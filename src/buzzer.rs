use crate::audio_logic::{AlarmTone, Tone};
use defmt::{info, warn};
use embassy_executor::{task, SpawnError, Spawner};
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Instant, Timer};
use esp_hal::gpio::Output;

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

pub fn spawn(spawner: &Spawner, pin: Output<'static>) -> Result<(), SpawnError> {
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
                info!("buzzer.alarm: active={}", alarm_label(*alarm));
            }
            None
        }
    }
}

#[task]
async fn task(mut pin: Output<'static>) {
    pin.set_low();
    let rx = COMMANDS.receiver();
    let mut alarm: Option<AlarmTone> = None;
    let mut pending_tone: Option<Tone> = None;

    loop {
        if alarm.is_none() {
            if let Some(tone) = pending_tone.take() {
                info!("buzzer.play: tone={}", tone.label());
                play_events(&mut pin, events_for_tone(tone)).await;
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
                play_events(&mut pin, events_for_alarm(active_alarm)).await;
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
            play_events(&mut pin, events_for_tone(tone)).await;
        }
    }
}

async fn play_events(pin: &mut Output<'static>, events: &[Event]) {
    for event in events {
        if event.freq_hz == 0 {
            pin.set_low();
            Timer::after(Duration::from_millis(event.ms as u64)).await;
        } else {
            play_square(pin, event.freq_hz, event.ms).await;
        }
    }
    pin.set_low();
}

async fn play_square(pin: &mut Output<'static>, freq_hz: u16, ms: u16) {
    let half_period_us = 500_000u64 / u64::from(freq_hz.max(1));
    let end = Instant::now() + Duration::from_millis(ms as u64);
    while Instant::now() < end {
        pin.set_high();
        Timer::after(Duration::from_micros(half_period_us)).await;
        pin.set_low();
        Timer::after(Duration::from_micros(half_period_us)).await;
    }
    pin.set_low();
}
