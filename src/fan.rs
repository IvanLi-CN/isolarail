use defmt::{info, warn};
use embassy_executor::{task, SpawnError, Spawner};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Input, InputConfig, Level, Output, Pull};
use esp_hal::ledc::channel::ChannelIFace;
use esp_hal::ledc::timer::TimerIFace;
use esp_hal::ledc::{channel, timer, LSGlobalClkSource, Ledc, LowSpeed};
use esp_hal::pcnt::{channel as pcnt_channel, unit, Pcnt};
use esp_hal::peripherals::APB_SARADC as _;
use esp_hal::peripherals::SENS as _; // for SENS::regs()
use esp_hal::peripherals::SYSTEM as _; // for SYSTEM::regs()
use esp_hal::time::Rate; // for APB_SARADC::regs()
#[allow(improper_ctypes)]
extern "C" {
    fn esp_rom_regi2c_read(block: u8, block_hostid: u8, reg_add: u8) -> u8;
    fn rom_i2c_writeReg(block: u8, block_hostid: u8, reg_add: u8, indata: u8);
}

// GPIO mapping per docs/esp32-s3fh4r2_gpio_assignment_guide.md
// - GPIO1:  FAN_PWM (LEDC 25 kHz, 三线风扇DC调速，经LDO/PWM调压)
// - GPIO2:  FAN_EN  (数字使能，高=开)
// - GPIO6:  FAN_TACH (PCNT测速)

const TACH_PULSES_PER_REV: u32 = 2; // common PC fan default

// 轮询 PCNT 的最小等待粒度（非固定窗，仅用于避免忙等）
const RPM_POLL_MS: u64 = 5;
// 单次样本的最长等待（避免风扇/测速线异常时阻塞）：
const RPM_SAMPLE_TIMEOUT_MS_FAST: u64 = 250; // 运行期
const RPM_SAMPLE_TIMEOUT_MS_CAL: u64 = 300; // 校准期
                                            // 连续样本的中位数窗口
const RPM_MEDIAN_K_FAST: usize = 3;
const RPM_MEDIAN_K_CAL: usize = 7;
// 平台稳定判据：连续 N 次中位数无显著上升（仅校准使用）
const CALIB_STABLE_REQUIRED: usize = 12; // 连续若干次稳定（无需最小采集次数）
const CALIB_STABLE_EPS_PCT: u32 = 2; // 允许变化占比
const CALIB_STABLE_EPS_ABS: u32 = 50; // 或绝对值
                                      // Control loop tick
const CTRL_TICK_MS: u64 = 500;
// 校准观测策略：不假设转速生效时间，直接连续取样并检测稳定
const CALIB_SPINUP_MS: u64 = 0; // 不预设加速时间；稳定性判据自行收敛
                                // 校准“观测预算”上限（非固定采样窗；达到平台可提前结束）
const CALIB_OBSERVE_MAX_MS: u64 = 6000; // 仅兜底超时，避免卡住固件

// Temperature control policy
const TEMP_TARGET_C: f32 = 50.0; // keep below this
const TEMP_FAN_START_C: f32 = 40.0; // begin ramp here
                                    // 满速温度改为 50°C：要求“将温度控制在 50 度以内”。
const TEMP_FAN_FULL_C: f32 = 50.0; // full speed here
const TEMP_FORCE_FULL_C: f32 = 80.0; // safety: 强制满速阈值
const TEMP_OFF_HYST_C: f32 = 1.5; // hysteresis when turning off (currently unused)
const MIN_DUTY_PCT: u8 = 0; // per requirement: allow 0%
                            // TSENS conversion constants (ESP32-S3):
                            // General linear model from Espressif docs:
                            //   T(°C) = 0.4386 * raw - 27.88 * dac_offset - 20.52
                            // 其中 dac_offset 由硬件寄存器 I2C_SARADC_TSENS_DAC(3:0) 决定。
                            // 我们不做“校准”，而是：
                            //   1) 上电将 TSENS_DAC 设为 1（常用量程，对应 dac_offset = -1）
                            //   2) 计算时从寄存器读回 TSENS_DAC，并按 dac_offset = - (reg & 0x0F) 使用。
const TSENS_ADC_FACTOR: f32 = 0.4386;
const TSENS_DAC_FACTOR: f32 = 27.88;
const TSENS_SYS_OFFSET: f32 = 20.52;

pub fn spawn(
    spawner: &Spawner,
    ledc: esp_hal::peripherals::LEDC<'static>,
    pcnt: esp_hal::peripherals::PCNT<'static>,
    sens: esp_hal::peripherals::SENS<'static>,
    pwm_pin: esp_hal::peripherals::GPIO1<'static>,
    en_pin: esp_hal::peripherals::GPIO2<'static>,
    tach_pin: esp_hal::peripherals::GPIO6<'static>,
) -> Result<(), SpawnError> {
    spawner.spawn(task(ledc, pcnt, sens, pwm_pin, en_pin, tach_pin))
}

#[task]
async fn task(
    ledc_dev: esp_hal::peripherals::LEDC<'static>,
    pcnt_dev: esp_hal::peripherals::PCNT<'static>,
    _sens_dev: esp_hal::peripherals::SENS<'static>,
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

    // Create channel on GPIO1（DC 调速：需要推挽输出，给 LDO/PWM 控制脚提供完整电平）
    let mut ch0 = ledc.channel(channel::Number::Channel0, pwm_pin);
    let _ = ch0.configure(channel::config::Config {
        timer: &lst,
        duty_pct: 0,
        pin_config: channel::config::PinConfig::PushPull,
    });
    info!("fan.pwm: ch0 cfg ok on GPIO1 (push-pull)");

    // PCNT tachometer setup on Unit0/Channel0, count rising edges
    let cfg = InputConfig::default().with_pull(Pull::Up);
    let tach_gpio = Input::new(tach_pin, cfg);
    let tach_in = tach_gpio.peripheral_input();
    let pcnt = Pcnt::new(pcnt_dev);
    let u0 = &pcnt.unit0;
    u0.set_low_limit(Some(-32_000)).ok();
    u0.set_high_limit(Some(32_000)).ok();
    // Enable digital filter to deglitch tach pulses (~10us at APB 80MHz)
    // 强化数字滤波以抑制供电/EMI耦合导致的毛刺计数：
    // 约 5,000 个 APB 周期（APB 80MHz 时 ~62.5µs）。
    // 标准 PC 风扇转速信号频率远低于此阈值。
    u0.set_filter(Some(5000)).ok();
    u0.clear();
    let ch = &u0.channel0;
    ch.set_ctrl_mode(pcnt_channel::CtrlMode::Keep, pcnt_channel::CtrlMode::Keep);
    ch.set_input_mode(
        pcnt_channel::EdgeMode::Hold,
        pcnt_channel::EdgeMode::Increment,
    );
    ch.set_edge_signal(tach_in);
    u0.resume();

    // Power up the on-chip temperature sensor (SENS block) and basic config
    // Enable TSENS clocks via SYSTEM and SENS blocks, and ensure SAR ADC timer runs
    let sys = esp_hal::peripherals::SYSTEM::regs();
    // Enable APB_SARADC peripheral clock (harmless; TSENS uses SENS gate below)
    sys.perip_clk_en0()
        .modify(|_, w| w.apb_saradc_clk_en().set_bit());

    // Enable TSENS peripheral clock gate at SENS domain (per IDF LL)
    let sens = esp_hal::peripherals::SENS::regs();
    sens.sar_meas1_mux()
        .modify(|_, w| w.sar1_dig_force().set_bit());
    sens.sar_meas2_mux()
        .modify(|_, w| w.sar2_rtc_force().clear_bit());

    let regs = esp_hal::peripherals::SENS::regs();
    regs.sar_peri_clk_gate_conf()
        .modify(|_, w| w.tsens_clk_en().set_bit());
    // Force TSENS XPD in CTRL2 and keep clock polarity default
    regs.sar_tsens_ctrl2()
        .modify(|_, w| unsafe { w.sar_tsens_xpd_force().bits(3) });
    regs.sar_tsens_ctrl()
        .modify(|_, w| w.sar_tsens_power_up().set_bit());
    // Force mode: sensor powered and conversions triggered via DUMP_OUT
    regs.sar_tsens_ctrl()
        .modify(|_, w| w.sar_tsens_power_up_force().set_bit());
    regs.sar_tsens_ctrl()
        .modify(|_, w| unsafe { w.sar_tsens_clk_div().bits(6) });
    // Enable REGI2C TSENS path: set ADC_SAR_ENT_TSENS (I2C_SAR_REG7 bit2) = 1
    unsafe {
        let v = esp_rom_regi2c_read(0x69, 1, 0x07);
        rom_i2c_writeReg(0x69, 1, 0x07, v | (1 << 2));
    }
    // Select standard range: TSENS_DAC = 1 (interpreted as dac_offset = -1)
    unsafe {
        let cur = esp_rom_regi2c_read(0x69, 1, 0x06) & 0xF;
        if cur != 0x1 {
            rom_i2c_writeReg(0x69, 1, 0x06, (cur & !0xF) | 0x1);
        }
    }
    // Allow sensor to stabilize
    Timer::after(Duration::from_micros(200)).await;
    // One-shot diagnostics
    {
        let c = regs.sar_tsens_ctrl().read();
        let c2 = regs.sar_tsens_ctrl2().read();
        let g = regs.sar_peri_clk_gate_conf().read();
        let r7 = unsafe { esp_rom_regi2c_read(0x69, 1, 0x07) };
        let r6 = unsafe { esp_rom_regi2c_read(0x69, 1, 0x06) } & 0xF;
        let ent_tsens = (r7 >> 2) & 1;
        info!(
            "tsens.diag: clk_en={} pu={} pu_force={} ready={} clk_div={} xpd_force={} raw={} ent_tsens={} tsens_dac=0x{:X}",
            g.tsens_clk_en().bit() as u8,
            c.sar_tsens_power_up().bit() as u8,
            c.sar_tsens_power_up_force().bit() as u8,
            c.sar_tsens_ready().bit() as u8,
            c.sar_tsens_clk_div().bits(),
            c2.sar_tsens_xpd_force().bits(),
            c.sar_tsens_out().bits(),
            ent_tsens,
            r6,
        );
    }

    // Temperature EMA & sampling heartbeats
    let mut ema_temp: Option<f32> = None;
    let mut ts_seq: u32 = 0;
    let mut last_raw: u8 = 0xFF;
    let mut same_count: u32 = 0;

    // ----- 上电满速校准（仅依据“转起来后的”连续采样中位数与稳定性）-----
    let _ = fan_en.set_high();
    set_speed_pct(&ch0, 100);
    let calib = measure_max_rpm_diag(u0, CALIB_SPINUP_MS, CALIB_OBSERVE_MAX_MS).await;
    let tach_valid = calib.valid;
    let max_rpm = calib.rpm;
    info!(
        "fan.calib: max_rpm={} valid={} reason={} elapsed={}ms pulses={} jitter={}pct samples={}",
        calib.rpm,
        tach_valid as u8,
        if calib.reason_timeout {
            "deadline"
        } else {
            "plateau"
        },
        calib.elapsed_ms,
        calib.total_pulses,
        calib.best_range_pct,
        calib.samples
    );
    // No forced stop after calibration; control loop will decide.

    // Control state for PI
    let mut duty_pi: i32 = 0;
    let mut integral: i32 = 0;
    // applied_duty 表示“速度百分比”0..100（与硬件占空相反）
    let mut applied_duty: u8 = 0;
    // 运行期观测到的最大转速（用于修正上电标定偏低）
    let mut max_rpm_seen: u32 = 0;

    let mut off_log_ms: u32 = 0;
    loop {
        // Temperature read & smooth
        let (raw, t_c) = read_temp_c_raw_conv().await;
        ts_seq = ts_seq.wrapping_add(1);
        // 仅在 raw 变化或 stuck 时打印，减少日志噪声
        if raw != last_raw {
            let ent = unsafe { (esp_rom_regi2c_read(0x69, 1, 0x07) >> 2) & 1 };
            info!(
                "tsens.raw_change: seq={} raw={} ent_tsens={}",
                ts_seq, raw, ent
            );
        }
        if raw == last_raw {
            same_count = same_count.saturating_add(1);
            if same_count % 10 == 0 {
                info!(
                    "tsens.stuck: raw_unmoved_ms={} last={}",
                    same_count * (CTRL_TICK_MS as u32),
                    raw
                );
            }
        } else {
            same_count = 0;
            last_raw = raw;
        }
        ema_temp = Some(match ema_temp {
            None => t_c,
            Some(prev) => 0.3 * t_c + 0.7 * prev,
        });
        let t = ema_temp.unwrap();

        // Target speed percentage from temperature（含安全阈值）
        let target_pct = if t >= TEMP_FORCE_FULL_C {
            100
        } else {
            temp_to_target_pct(t)
        };
        if t >= TEMP_FORCE_FULL_C {
            info!("fan.safety: T={}.1C force_full", t as i32);
        }

        if target_pct == 0 {
            // Off state
            integral = 0;
            duty_pi = 0;
            applied_duty = 0;
            set_speed_pct(&ch0, 0);
            let _ = fan_en.set_low();
            off_log_ms += CTRL_TICK_MS as u32;
            if off_log_ms >= 1000 {
                off_log_ms = 0;
                info!(
                    "fan.temp: T={}.1C raw={} tgt_pct=0 state=off",
                    t as i32, raw
                );
            }
        } else if !tach_valid || max_rpm == 0 {
            // 无（或不可信）转速信号：保持满速
            let _ = fan_en.set_high();
            applied_duty = 100; // 立即满速，不做缓升
            set_speed_pct(&ch0, applied_duty);
            // 运行期取连续小样本的中位数，过滤异常
            let mut buf: [u32; RPM_MEDIAN_K_FAST] = [0; RPM_MEDIAN_K_FAST];
            let mut got = 0usize;
            for i in 0..RPM_MEDIAN_K_FAST {
                if let Some((_ms, rpm, _p)) = rpm_sample_once(u0, RPM_SAMPLE_TIMEOUT_MS_FAST).await
                {
                    buf[i] = rpm;
                    got += 1;
                } else {
                    break;
                }
            }
            let rpm = if got > 0 {
                median_in_place(&mut buf[..got]) as i32
            } else {
                0
            };
            info!(
                "fan.fail_tach: force_max duty={} rpm={} (tach_valid={})",
                applied_duty, rpm, tach_valid as u8
            );
        } else {
            // Closed-loop PI to reach target RPM
            let _ = fan_en.set_high();
            let mut buf2: [u32; RPM_MEDIAN_K_FAST] = [0; RPM_MEDIAN_K_FAST];
            let mut got2 = 0usize;
            for i in 0..RPM_MEDIAN_K_FAST {
                if let Some((_ms, rpm, _p)) = rpm_sample_once(u0, RPM_SAMPLE_TIMEOUT_MS_FAST).await
                {
                    buf2[i] = rpm;
                    got2 += 1;
                } else {
                    break;
                }
            }
            let rpm = if got2 > 0 {
                median_in_place(&mut buf2[..got2]) as i32
            } else {
                0
            };
            // 按要求：测速可信性仅在初始化阶段判定；运行期不再否定。
            // 此处不再根据“低占空高转速”去修改 tach_valid。
            // 在线修正最大转速
            if rpm > 0 {
                max_rpm_seen = max_rpm_seen.max(rpm as u32);
            }
            let eff_max_rpm = (max_rpm_seen.max(max_rpm)).max(1);
            let target_rpm = (eff_max_rpm as i32) * (target_pct as i32) / 100;

            // PI gains
            const KP: i32 = 50;
            const KI: i32 = 6;

            let err = (target_rpm - rpm).clamp(-(max_rpm as i32), max_rpm as i32);
            integral = (integral + err).clamp(-3 * max_rpm as i32, 3 * max_rpm as i32);
            let step_p = (err * KP) / (max_rpm as i32).max(1);
            let step_i = (integral * KI) / (max_rpm as i32).max(1);
            duty_pi = (duty_pi + step_p + step_i).clamp(0, 100);
            let desired = duty_pi as u8;

            applied_duty = slew(applied_duty, desired, 6);
            set_speed_pct(&ch0, applied_duty);
            let hw_duty = speed_to_hw_duty(applied_duty);

            info!(
                "fan.pi: T={}.1C raw={} tgt_pct={} duty={} rpm={} tgt_rpm={} hw={} max={}",
                t as i32, raw, target_pct, applied_duty, rpm, target_rpm, hw_duty, eff_max_rpm
            );
            off_log_ms = 0;
        }

        Timer::after(Duration::from_millis(CTRL_TICK_MS)).await;
    }
}

// 自适应测速：累计达到最小转数或超时即结算
// 返回 (实际测量时长ms, RPM)
// 一次“无窗”样本：清零 → 等待到至少一脉冲或超时 → 计算 Δpulses/Δt。
// 返回 Some((elapsed_ms, rpm, pulses))；若超时且无脉冲则返回 None。
async fn rpm_sample_once(
    u0: &unit::Unit<'_, 0>,
    sample_timeout_ms: u64,
) -> Option<(u64, u32, u32)> {
    u0.clear();
    let mut elapsed_ms: u64 = 0;
    loop {
        Timer::after(Duration::from_millis(RPM_POLL_MS)).await;
        elapsed_ms += RPM_POLL_MS;
        let pulses = u0.value() as i32;
        let pulses = pulses.max(0) as u32;
        if pulses > 0 {
            let rpm = (pulses as u64).saturating_mul(60_000)
                / (elapsed_ms as u64).max(1)
                / (TACH_PULSES_PER_REV as u64);
            return Some((elapsed_ms, (rpm as u32).min(12_000), pulses));
        }
        if elapsed_ms >= sample_timeout_ms {
            return None;
        }
    }
}

fn median_in_place(v: &mut [u32]) -> u32 {
    v.sort_unstable();
    v[v.len() / 2]
}

// Measure max RPM using sliding-window average with spin-up and plateau detection
// 校准：带诊断信息的最大转速测量
struct CalibDiag {
    rpm: u32,
    valid: bool,
    elapsed_ms: u64,
    reason_timeout: bool,
    total_pulses: u32,
    best_range_pct: u32,
    samples: u32,
}

async fn measure_max_rpm_diag(
    u0: &unit::Unit<'_, 0>,
    spinup_ms: u64,
    observe_max_ms: u64,
) -> CalibDiag {
    // 满速拉起（不假设风扇加速时间，只做最小等待以避免极短区间）
    Timer::after(Duration::from_millis(spinup_ms)).await;

    let t0 = esp_hal::time::Instant::now();
    let mut elapsed_ms: u64 = 0;
    let mut total_pulses: u32 = 0;
    let mut samples: u32 = 0;
    let mut max_rpm: u32 = 0;
    let mut med_prev: Option<u32> = None;
    let mut stable_streak: usize = 0;
    let mut jitter_pct: u32 = 100;
    let mut reason_timeout = true;

    while elapsed_ms < observe_max_ms {
        // 采集一组样本并取中位数（过滤离群）。仅以“总观测超时”作为结束条件。
        let mut buf: [u32; RPM_MEDIAN_K_CAL] = [0; RPM_MEDIAN_K_CAL];
        let mut got = 0usize;
        for i in 0..RPM_MEDIAN_K_CAL {
            if let Some((_ms, rpm, pulses)) = rpm_sample_once(u0, RPM_SAMPLE_TIMEOUT_MS_CAL).await {
                buf[i] = rpm;
                got += 1;
                total_pulses = total_pulses.saturating_add(pulses);
            } else {
                break; // 无脉冲：可能无测速线或未达有效边沿
            }
        }
        elapsed_ms = (esp_hal::time::Instant::now() - t0).as_millis();
        if got == 0 {
            break;
        }
        let med = median_in_place(&mut buf[..got]);
        samples = samples.saturating_add(1);
        if med > max_rpm {
            max_rpm = med;
        }

        // 组内抖动（max-min 占比），仅用于日志
        let minv = *buf[..got].iter().min().unwrap();
        let maxv = *buf[..got].iter().max().unwrap();
        jitter_pct = if med > 0 {
            ((maxv.saturating_sub(minv)) * 100) / med
        } else {
            100
        };

        // 稳定性：连续 CALIB_STABLE_REQUIRED 次中位数未显著上升即可判稳
        if let Some(prev) = med_prev {
            let eps = (prev * CALIB_STABLE_EPS_PCT) / 100 + CALIB_STABLE_EPS_ABS;
            if med <= prev + eps {
                stable_streak = stable_streak.saturating_add(1);
            } else {
                stable_streak = 0;
            }
        }
        med_prev = Some(med);
        if stable_streak >= CALIB_STABLE_REQUIRED {
            reason_timeout = false;
            break;
        }
    }

    CalibDiag {
        rpm: max_rpm,
        // 只有在“非 deadline（即 plateau 收敛）”时才认定有效，符合“最大转速必须由稳定确认”
        valid: max_rpm > 0 && !reason_timeout,
        elapsed_ms,
        reason_timeout,
        total_pulses,
        best_range_pct: jitter_pct,
        samples,
    }
}

// 三线 DC 调速在本板上为“有效低”（0% duty → 最高电压/最高转速）。
// 因此：速度百分比 100% → 硬件占空 0%；速度 0% → 硬件占空 100%。
fn speed_to_hw_duty(speed_pct: u8) -> u8 {
    100u8.saturating_sub(speed_pct.min(100))
}
fn set_speed_pct(ch0: &esp_hal::ledc::channel::Channel<'_, LowSpeed>, speed_pct: u8) {
    let hw = speed_to_hw_duty(speed_pct);
    let _ = ch0.set_duty(hw);
}

async fn read_temp_c_raw_conv() -> (u8, f32) {
    let regs = esp_hal::peripherals::SENS::regs();

    // One-shot conversion sequence using DUMP_OUT
    regs.sar_tsens_ctrl()
        .modify(|_, w| w.sar_tsens_dump_out().set_bit());
    // 给一次触发-采样-转储一点准备时间
    Timer::after(Duration::from_micros(100)).await;

    let mut raw: u8 = 0;
    let mut tries = 0;
    loop {
        let r = regs.sar_tsens_ctrl().read();
        if r.sar_tsens_ready().bit() {
            raw = r.sar_tsens_out().bits();
            break;
        }
        tries += 1;
        if tries >= 40 {
            raw = r.sar_tsens_out().bits();
            break;
        }
        Timer::after(Duration::from_micros(200)).await;
    }
    regs.sar_tsens_ctrl()
        .modify(|_, w| w.sar_tsens_dump_out().clear_bit());

    // Read current TSENS_DAC value and derive dac_offset as a negative integer
    let tsens_dac = unsafe { esp_rom_regi2c_read(0x69, 1, 0x06) } & 0x0F;
    let dac_offset: i8 = -(tsens_dac as i8);
    let raw_f = raw as f32;
    let t_c =
        TSENS_ADC_FACTOR * raw_f - (TSENS_DAC_FACTOR * (dac_offset as f32)) - TSENS_SYS_OFFSET;
    (raw, t_c)
}

fn temp_to_target_pct(temp_c: f32) -> u8 {
    if temp_c <= TEMP_FAN_START_C {
        return 0; // 40°C 以下不转
    }
    if temp_c >= TEMP_FAN_FULL_C {
        return 100;
    }
    let span = TEMP_FAN_FULL_C - TEMP_FAN_START_C;
    let pos = (temp_c - TEMP_FAN_START_C) / span;
    (pos * 100.0).clamp(0.0, 100.0) as u8
}

fn slew(current: u8, target: u8, max_step: u8) -> u8 {
    if target > current {
        current.saturating_add((target - current).min(max_step))
    } else {
        current.saturating_sub((current - target).min(max_step))
    }
}
