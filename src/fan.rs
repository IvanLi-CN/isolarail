use defmt::info;
use embassy_executor::{task, SpawnError, Spawner};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Input, InputConfig, Level, Output, Pull};
use esp_hal::ledc::channel::ChannelIFace;
use esp_hal::ledc::timer::TimerIFace;
use esp_hal::ledc::{channel, timer, LSGlobalClkSource, Ledc, LowSpeed};
use esp_hal::pcnt::{channel as pcnt_channel, unit, Pcnt};
use esp_hal::time::Rate;

// GPIO mapping per docs/esp32-s3fh4r2_gpio_assignment_guide.md
// - GPIO1:  FAN_PWM (LEDC low-speed 25 kHz)
// - GPIO2:  FAN_EN  (digital enable, high=on)
// - GPIO6:  FAN_TACH (PCNT input)

const TACH_PULSES_PER_REV: u32 = 2; // common PC fan default

// Sampling configuration for RPM measurement
const SAMPLE_MS: u64 = 200;
// Control loop tick and duration
const CTRL_TICK_MS: u64 = 500;
const STABLE_WINDOW: usize = 6; // consecutive samples
const STABLE_TOL_PCT: u32 = 3; // within +/-3%

pub fn spawn(
    spawner: &Spawner,
    ledc: esp_hal::peripherals::LEDC<'static>,
    pcnt: esp_hal::peripherals::PCNT<'static>,
    pwm_pin: esp_hal::peripherals::GPIO1<'static>,
    en_pin: esp_hal::peripherals::GPIO2<'static>,
    tach_pin: esp_hal::peripherals::GPIO6<'static>,
) -> Result<(), SpawnError> {
    spawner.spawn(task(ledc, pcnt, pwm_pin, en_pin, tach_pin))
}

#[task]
async fn task(
    ledc_dev: esp_hal::peripherals::LEDC<'static>,
    pcnt_dev: esp_hal::peripherals::PCNT<'static>,
    pwm_pin: esp_hal::peripherals::GPIO1<'static>,
    en_pin: esp_hal::peripherals::GPIO2<'static>,
    tach_pin: esp_hal::peripherals::GPIO6<'static>,
) {
    // Enable pin default off
    let mut fan_en = Output::new(en_pin, Level::Low, esp_hal::gpio::OutputConfig::default());

    // LEDC PWM setup: LowSpeed timer @ 25 kHz, 13-bit resolution
    let mut ledc = Ledc::new(ledc_dev);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut lst = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    // 25 kHz @ 13-bit requires divisor < 1 (invalid). Use 10-bit for 25 kHz.
    if let Err(_) = lst.configure(timer::config::Config {
        duty: timer::config::Duty::Duty10Bit,
        clock_source: timer::LSClockSource::APBClk,
        frequency: Rate::from_khz(25),
    }) {
        // Fallback: 10 kHz @ 13-bit (always valid)
        let _ = lst.configure(timer::config::Config {
            duty: timer::config::Duty::Duty13Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(10),
        });
    }
    info!(
        "fan.pwm: timer cfg ok freq={}Hz duty_bits={}",
        lst.frequency(),
        lst.duty().map(|d| d as u32).unwrap_or(0)
    );

    // Create channel on GPIO1
    let mut ch0 = ledc.channel(channel::Number::Channel0, pwm_pin);
    let _ = ch0.configure(channel::config::Config {
        timer: &lst,
        duty_pct: 0,
        // PushPull: 0V/3.3V full-swing for easier probing (no external pull-up)
        pin_config: channel::config::PinConfig::PushPull,
    });
    info!("fan.pwm: ch0 cfg ok on GPIO1 (push-pull)");

    // PCNT tachometer setup on Unit0/Channel0, count rising edges
    let cfg = InputConfig::default().with_pull(Pull::Up);
    let tach_gpio = Input::new(tach_pin, cfg);
    let tach_in = tach_gpio.peripheral_input();
    let mut pcnt = Pcnt::new(pcnt_dev);
    let u0 = &pcnt.unit0;
    u0.set_low_limit(Some(-32_000)).ok();
    u0.set_high_limit(Some(32_000)).ok();
    // Enable digital filter to deglitch tach pulses (~10us at APB 80MHz)
    u0.set_filter(Some(800)).ok();
    u0.clear();
    let ch = &u0.channel0;
    ch.set_ctrl_mode(pcnt_channel::CtrlMode::Keep, pcnt_channel::CtrlMode::Keep);
    ch.set_input_mode(
        pcnt_channel::EdgeMode::Hold,
        pcnt_channel::EdgeMode::Increment,
    );
    ch.set_edge_signal(tach_in);
    u0.resume();

    // Bring fan up and calibrate max RPM at 0% pull-down (max supply)
    fan_en.set_high();
    set_pull_down(&ch0, 0);
    let max_rpm = measure_until_stable(u0, 3000).await;
    info!("fan.max_rpm={}", max_rpm);

    // Closed-loop test: track target speed percentages (10/50/100%)
    // Each step runs 5s with PI control using tach feedback.
    let steps = [10u8, 50u8, 100u8];
    let mut duty: i32 = 0; // pull-down percentage; start 0% after calibration
    loop {
        for pct in steps {
            if max_rpm == 0 {
                // Tach not available: degrade to open-loop duty set + log
                let _ = ch0.set_duty(pct);
                let rpm = measure_rpm_ms(u0, SAMPLE_MS).await;
                let pct_of_max = 0;
                info!("fan.set={}pct rpm={} pct_of_max={}", pct, rpm, pct_of_max);
                Timer::after(Duration::from_secs(5)).await;
                continue;
            }
            let target_rpm = (max_rpm as u32 * pct as u32 / 100) as i32;
            duty = control_to_target_pi(&ch0, u0, target_rpm, max_rpm as i32, duty, 5000).await;
        }
    }
}

async fn measure_until_stable(u0: &unit::Unit<'_, 0>, timeout_ms: u64) -> u32 {
    let mut window: [u32; STABLE_WINDOW] = [0; STABLE_WINDOW];
    let mut pos = 0usize;
    let mut filled = 0usize;
    let mut elapsed_ms: u64 = 0;

    // give fan time to spin up
    Timer::after(Duration::from_millis(400)).await;
    elapsed_ms += 400;

    loop {
        let rpm = measure_rpm_ms(u0, SAMPLE_MS).await;
        window[pos] = rpm;
        pos = (pos + 1) % STABLE_WINDOW;
        filled = filled.saturating_add(1).min(STABLE_WINDOW);
        elapsed_ms += SAMPLE_MS;

        if filled == STABLE_WINDOW {
            let min = *window.iter().min().unwrap();
            let max = *window.iter().max().unwrap();
            let span_pct = if max > 0 {
                ((max - min) * 100) / max
            } else {
                100
            };
            if span_pct <= STABLE_TOL_PCT {
                let avg = window.iter().copied().sum::<u32>() / (STABLE_WINDOW as u32);
                return avg;
            }
        }
        if elapsed_ms >= timeout_ms {
            // timeout, return best effort average (or 0)
            let avg = if filled > 0 {
                window.iter().take(filled).copied().sum::<u32>() / (filled as u32)
            } else {
                0
            };
            return avg;
        }
    }
}

async fn measure_rpm_ms(u0: &unit::Unit<'_, 0>, period_ms: u64) -> u32 {
    u0.clear();
    Timer::after(Duration::from_millis(period_ms)).await;
    let pulses = u0.value() as i32; // signed value
    let pulses = pulses.max(0) as u32;
    let revs = pulses / TACH_PULSES_PER_REV;
    let rpm = (revs as u64) * 60_000 / (period_ms as u64);
    rpm as u32
}

// Simple PI controller: adjusts duty to reach target RPM within the given duration.
async fn control_to_target_pi(
    ch0: &esp_hal::ledc::channel::Channel<'_, LowSpeed>,
    u0: &unit::Unit<'_, 0>,
    target_rpm: i32,
    max_rpm: i32,
    mut duty_pct: i32,
    duration_ms: u64,
) -> i32 {
    // Gains tuned conservatively for stability; integer math
    // step = Kp*err/max + Ki*sum_err/max
    const KP: i32 = 40; // proportional gain
    const KI: i32 = 6; // integral gain
    const MIN_DUTY: i32 = 5; // avoid stall for non-zero targets

    let mut integral: i32 = 0;
    let mut elapsed: u64 = 0;
    let mut last_log_ms: u64 = 0;

    // Initial log
    let mut rpm = measure_rpm_ms(u0, CTRL_TICK_MS).await as i32;
    set_pull_down(ch0, duty_pct.clamp(0, 100) as u8);
    log_ctrl("start", target_rpm, rpm, duty_pct, max_rpm);

    let mut stagnant_ticks: u32 = 0;
    let mut last_rpm: i32 = rpm;
    while elapsed < duration_ms {
        rpm = measure_rpm_ms(u0, CTRL_TICK_MS).await as i32;

        // Error sign: positive when rpm is too high -> increase pull-down to slow the fan
        let err = (rpm - target_rpm).clamp(-max_rpm, max_rpm);
        integral = (integral + err).clamp(-3 * max_rpm, 3 * max_rpm);
        let step_p = (err * KP) / max_rpm.max(1);
        let step_i = (integral * KI) / max_rpm.max(1);
        duty_pct = (duty_pct + step_p + step_i).clamp(0, 100);

        // honor MIN_DUTY for non-zero target
        if target_rpm > 0 && duty_pct < MIN_DUTY {
            duty_pct = MIN_DUTY;
        }
        set_pull_down(ch0, duty_pct as u8);

        elapsed += CTRL_TICK_MS;
        last_log_ms += CTRL_TICK_MS;

        // If rpm is stagnant (within +/-2%) and still far above target, push duty up faster
        let stagnant = ((rpm - last_rpm).abs() * 100 / max_rpm.max(1)) <= 2;
        let far_above = rpm > target_rpm + (max_rpm / 20); // > +5% above target
        if stagnant && far_above {
            stagnant_ticks += 1;
            if stagnant_ticks >= 3 {
                duty_pct = (duty_pct + 10).clamp(0, 100);
                set_pull_down(ch0, duty_pct as u8);
                stagnant_ticks = 0;
            }
        } else {
            stagnant_ticks = 0;
        }
        last_rpm = rpm;
        if last_log_ms >= 1000 {
            last_log_ms = 0;
            log_ctrl("loop", target_rpm, rpm, duty_pct, max_rpm);
        }
    }

    // Final report for this step
    log_ctrl("done", target_rpm, rpm, duty_pct, max_rpm);
    duty_pct
}

fn log_ctrl(tag: &str, target_rpm: i32, rpm: i32, duty_pct: i32, max_rpm: i32) {
    let pct_of_max = if max_rpm > 0 {
        (rpm.max(0) * 100 / max_rpm)
    } else {
        0
    };
    info!(
        "fan.ctrl:{} target={}rpm duty={}pct rpm={} pct_of_max={}",
        tag,
        target_rpm,
        duty_pct,
        rpm.max(0),
        pct_of_max
    );
}

// Map logical pull-down percentage to LEDC duty under OpenDrain.
// OpenDrain: duty% describes HIGH time (Hi-Z). We want LOW time ratio,
// so set duty = 100 - pull_down%.
// Under PushPull: low-time fraction equals pull-down percentage directly.
fn set_pull_down(ch0: &esp_hal::ledc::channel::Channel<'_, LowSpeed>, pull_down_pct: u8) {
    let _ = ch0.set_duty(pull_down_pct.min(100));
}
