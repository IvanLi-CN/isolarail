use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, Ordering};

use crate::audio_logic::{AlarmTone, Tone};
use crate::hardware_snapshot as hwdiag;
use defmt::{info, warn};
use embassy_executor::{task, SpawnError, Spawner};
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_hal::clock::Clocks;
use esp_hal::ledc::channel::ChannelIFace;
use esp_hal::ledc::timer::TimerIFace;
use esp_hal::ledc::{channel, timer, LowSpeed};
use esp_hal::peripherals::{GPIO7, LEDC};
use esp_hal::time::Rate;

#[derive(Copy, Clone)]
struct Event {
    freq_hz: u16,
    ms: u16,
}

#[derive(Copy, Clone)]
enum Command {
    Play(Tone),
    SetAlarm(Option<AlarmTone>),
}

static COMMANDS: Channel<CriticalSectionRawMutex, Command, 16> = Channel::new();
static DRIVER_READY: AtomicBool = AtomicBool::new(false);
static PLAYING: AtomicBool = AtomicBool::new(false);
static ACTIVE_TONE: AtomicU8 = AtomicU8::new(0);
static ACTIVE_ALARM: AtomicU8 = AtomicU8::new(0);
static FREQUENCY_HZ: AtomicU16 = AtomicU16::new(0);
static DUTY_PCT: AtomicU8 = AtomicU8::new(0);
const BUZZER_TIMER: timer::Number = timer::Number::Timer1;
const BUZZER_CHANNEL: channel::Number = channel::Number::Channel1;
const BUZZER_DUTY: timer::config::Duty = timer::config::Duty::Duty10Bit;
const BUZZER_DUTY_BITS: u8 = 10;
const BUZZER_ON_DUTY_PCT: u8 = 50;
const LEDC_TIMER_DIV_NUM_MAX: u64 = 0x3FFFF;

const fn tone(freq_hz: u16, ms: u16) -> Event {
    Event { freq_hz, ms }
}

const fn rest(ms: u16) -> Event {
    Event { freq_hz: 0, ms }
}

const BOOT: &[Event] = &[
    tone(660, 70),
    rest(35),
    tone(880, 85),
    rest(35),
    tone(1320, 120),
];
const OPERATION_OK: &[Event] = &[tone(1047, 45), rest(25), tone(1319, 60)];
const OPERATION_DENIED: &[Event] = &[tone(440, 80), rest(35), tone(330, 110)];
const CHANNEL_POWER_ON: &[Event] = &[tone(784, 60), rest(30), tone(1175, 90)];
const CHANNEL_POWER_OFF: &[Event] = &[tone(1175, 55), rest(35), tone(784, 95)];
const HINT_CURRENT_3A: &[Event] = &[tone(880, 55), rest(40), tone(988, 55)];
const HINT_CURRENT_5A: &[Event] = &[tone(988, 60), rest(45), tone(784, 80)];
const HINT_INSERT: &[Event] = &[tone(784, 45), rest(25), tone(1047, 70)];
const HINT_REMOVE: &[Event] = &[tone(1047, 45), rest(30), tone(698, 80)];

const ALARM_OVER_TEMP: &[Event] = &[
    tone(1320, 70),
    rest(50),
    tone(1320, 70),
    rest(50),
    tone(988, 120),
];
const ALARM_INPUT_OVER_POWER: &[Event] = &[tone(740, 120), rest(70), tone(740, 120)];
const ALARM_CHANNEL_SHORT: &[Event] = &[
    tone(220, 70),
    rest(45),
    tone(220, 70),
    rest(45),
    tone(330, 70),
];
const ALARM_CHANNEL_OVER_5A: &[Event] = &[tone(988, 80), rest(80), tone(988, 80)];

pub fn spawn(spawner: &Spawner, pin: GPIO7<'static>) -> Result<(), SpawnError> {
    spawner.spawn(task(pin))
}

pub fn play(tone: Tone) {
    if COMMANDS.try_send(Command::Play(tone)).is_err() {
        warn!("buzzer.queue: drop tone={} reason=full", tone.label());
    }
}

pub fn set_alarm(alarm: Option<AlarmTone>) -> bool {
    if COMMANDS.try_send(Command::SetAlarm(alarm)).is_err() {
        warn!("buzzer.queue: drop alarm reason=full");
        false
    } else {
        true
    }
}

pub fn snapshot() -> hwdiag::BuzzerSnapshot {
    let ready = DRIVER_READY.load(Ordering::Relaxed);
    hwdiag::BuzzerSnapshot {
        state: if ready {
            hwdiag::NodeState::Online
        } else {
            hwdiag::NodeState::Error
        },
        driver_ready: ready,
        playing: PLAYING.load(Ordering::Relaxed),
        active_tone: tone_label_from_tag(ACTIVE_TONE.load(Ordering::Relaxed)),
        active_alarm: alarm_label_from_tag(ACTIVE_ALARM.load(Ordering::Relaxed)),
        frequency_hz: FREQUENCY_HZ.load(Ordering::Relaxed),
        duty_pct: DUTY_PCT.load(Ordering::Relaxed),
    }
}

fn events_for_tone(tone: Tone) -> &'static [Event] {
    match tone {
        Tone::Boot => BOOT,
        Tone::OperationOk => OPERATION_OK,
        Tone::OperationDenied => OPERATION_DENIED,
        Tone::ChannelPowerOn => CHANNEL_POWER_ON,
        Tone::ChannelPowerOff => CHANNEL_POWER_OFF,
        Tone::HintCurrent3A => HINT_CURRENT_3A,
        Tone::HintCurrent5A => HINT_CURRENT_5A,
        Tone::HintInsert => HINT_INSERT,
        Tone::HintRemove => HINT_REMOVE,
    }
}

fn events_for_alarm(alarm: AlarmTone) -> &'static [Event] {
    match alarm {
        AlarmTone::ChannelShort => ALARM_CHANNEL_SHORT,
        AlarmTone::OverTemp => ALARM_OVER_TEMP,
        AlarmTone::InputOverPower => ALARM_INPUT_OVER_POWER,
        AlarmTone::ChannelOver5A => ALARM_CHANNEL_OVER_5A,
    }
}

fn alarm_gap(alarm: AlarmTone) -> Duration {
    Duration::from_millis(match alarm {
        AlarmTone::ChannelShort => 220,
        AlarmTone::OverTemp => 280,
        AlarmTone::InputOverPower => 300,
        AlarmTone::ChannelOver5A => 1800,
    })
}

fn alarm_label(alarm: Option<AlarmTone>) -> &'static str {
    alarm.map(AlarmTone::label).unwrap_or("none")
}

fn tone_tag(tone: Tone) -> u8 {
    match tone {
        Tone::Boot => 1,
        Tone::OperationOk => 2,
        Tone::OperationDenied => 3,
        Tone::ChannelPowerOn => 4,
        Tone::ChannelPowerOff => 5,
        Tone::HintCurrent3A => 6,
        Tone::HintCurrent5A => 7,
        Tone::HintInsert => 8,
        Tone::HintRemove => 9,
    }
}

fn tone_label_from_tag(tag: u8) -> &'static str {
    match tag {
        1 => "boot",
        2 => "operation_ok",
        3 => "operation_denied",
        4 => "channel_power_on",
        5 => "channel_power_off",
        6 => "hint_current_3a",
        7 => "hint_current_5a",
        8 => "hint_insert",
        9 => "hint_remove",
        _ => "none",
    }
}

fn alarm_tag(alarm: AlarmTone) -> u8 {
    match alarm {
        AlarmTone::ChannelShort => 1,
        AlarmTone::OverTemp => 2,
        AlarmTone::InputOverPower => 3,
        AlarmTone::ChannelOver5A => 4,
    }
}

fn alarm_label_from_tag(tag: u8) -> &'static str {
    match tag {
        1 => "channel_short",
        2 => "over_temp",
        3 => "input_over_power",
        4 => "channel_over_5a",
        _ => "none",
    }
}

fn apply_command(command: Command, alarm: &mut Option<AlarmTone>) -> Option<Tone> {
    match command {
        Command::Play(tone) => {
            if alarm.is_some() {
                warn!(
                    "buzzer.play: tone={} deferred=false reason=alarm_active",
                    tone.label()
                );
                None
            } else {
                Some(tone)
            }
        }
        Command::SetAlarm(next) => {
            if *alarm != next {
                *alarm = next;
                ACTIVE_ALARM.store(next.map(alarm_tag).unwrap_or(0), Ordering::Relaxed);
                info!("buzzer.alarm: active={}", alarm_label(*alarm));
            }
            None
        }
    }
}

#[task]
async fn task(pin: GPIO7<'static>) {
    let mut pwm_timer = timer::Timer::<LowSpeed>::new(LEDC::regs(), BUZZER_TIMER);
    if pwm_timer
        .configure(timer::config::Config {
            duty: BUZZER_DUTY,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(1000),
        })
        .is_err()
    {
        warn!("buzzer.init: timer=fail");
        DRIVER_READY.store(false, Ordering::Relaxed);
        return;
    }

    let mut pwm_channel = channel::Channel::<LowSpeed>::new(BUZZER_CHANNEL, pin);
    if pwm_channel
        .configure(channel::config::Config {
            timer: &pwm_timer,
            duty_pct: 0,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .is_err()
    {
        warn!("buzzer.init: channel=fail");
        DRIVER_READY.store(false, Ordering::Relaxed);
        return;
    }
    DRIVER_READY.store(true, Ordering::Relaxed);
    info!("buzzer.init: driver=ledc timer=1 channel=1 gpio=GPIO7 idle=low");

    let rx = COMMANDS.receiver();
    let mut alarm: Option<AlarmTone> = None;
    let mut pending_tone: Option<Tone> = None;

    loop {
        if alarm.is_none() {
            if let Some(tone) = pending_tone.take() {
                info!("buzzer.play: tone={}", tone.label());
                ACTIVE_TONE.store(tone_tag(tone), Ordering::Relaxed);
                play_events(&pwm_channel, events_for_tone(tone)).await;
                ACTIVE_TONE.store(0, Ordering::Relaxed);
                continue;
            }
        }

        if alarm.is_some() {
            while let Ok(command) = rx.try_receive() {
                if let Some(tone) = apply_command(command, &mut alarm) {
                    pending_tone = Some(tone);
                    break;
                }
            }
            if alarm.is_none() && pending_tone.is_some() {
                continue;
            }
            if let Some(active_alarm) = alarm {
                info!("buzzer.alarm.play: tone={}", active_alarm.label());
                play_events(&pwm_channel, events_for_alarm(active_alarm)).await;
                match select(rx.receive(), Timer::after(alarm_gap(active_alarm))).await {
                    Either::First(command) => {
                        if let Some(tone) = apply_command(command, &mut alarm) {
                            pending_tone = Some(tone);
                        }
                    }
                    Either::Second(()) => {}
                }
            }
            continue;
        }

        if let Some(tone) = apply_command(rx.receive().await, &mut alarm) {
            info!("buzzer.play: tone={}", tone.label());
            ACTIVE_TONE.store(tone_tag(tone), Ordering::Relaxed);
            play_events(&pwm_channel, events_for_tone(tone)).await;
            ACTIVE_TONE.store(0, Ordering::Relaxed);
        }
    }
}

async fn play_events(channel: &channel::Channel<'_, LowSpeed>, events: &[Event]) {
    for event in events {
        if event.freq_hz == 0 {
            stop_pwm(channel);
            Timer::after(Duration::from_millis(event.ms as u64)).await;
        } else {
            play_square(channel, event.freq_hz, event.ms).await;
        }
    }
    stop_pwm(channel);
}

async fn play_square(channel: &channel::Channel<'_, LowSpeed>, freq_hz: u16, ms: u16) {
    stop_pwm(channel);
    if configure_pwm_frequency(freq_hz) {
        let _ = channel.set_duty(BUZZER_ON_DUTY_PCT);
        FREQUENCY_HZ.store(freq_hz, Ordering::Relaxed);
        DUTY_PCT.store(BUZZER_ON_DUTY_PCT, Ordering::Relaxed);
        PLAYING.store(true, Ordering::Relaxed);
        Timer::after(Duration::from_millis(ms as u64)).await;
        stop_pwm(channel);
    } else {
        warn!("buzzer.ledc: freq={}Hz configure=fail", freq_hz);
        Timer::after(Duration::from_millis(ms as u64)).await;
    }
}

fn stop_pwm(channel: &channel::Channel<'_, LowSpeed>) {
    let _ = channel.set_duty(0);
    PLAYING.store(false, Ordering::Relaxed);
    FREQUENCY_HZ.store(0, Ordering::Relaxed);
    DUTY_PCT.store(0, Ordering::Relaxed);
}

fn configure_pwm_frequency(freq_hz: u16) -> bool {
    let freq_hz = u32::from(freq_hz.max(1));
    let src_freq = Clocks::get().apb_clock.as_hz();
    let precision = 1u64 << BUZZER_DUTY_BITS;
    let divisor = ((u64::from(src_freq)) << 8) / u64::from(freq_hz) / precision;
    if !(256..LEDC_TIMER_DIV_NUM_MAX).contains(&divisor) {
        return false;
    }

    let ledc = LEDC::regs();
    ledc.timer(BUZZER_TIMER as usize)
        .conf()
        .modify(|_, w| unsafe {
            w.tick_sel().clear_bit();
            w.rst().clear_bit();
            w.pause().clear_bit();
            w.clk_div().bits(divisor as u32);
            w.duty_res().bits(BUZZER_DUTY_BITS)
        });
    ledc.timer(BUZZER_TIMER as usize)
        .conf()
        .modify(|_, w| w.para_up().set_bit());
    true
}
