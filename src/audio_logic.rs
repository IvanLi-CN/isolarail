#![allow(clippy::float_cmp)]

pub const INPUT_OVER_POWER_SET_W: f32 = 100.0;
pub const INPUT_OVER_POWER_CLEAR_W: f32 = 90.0;

pub const PORT_INSERT_MV: u32 = 3300;
pub const PORT_REMOVE_MV: u32 = 3000;
pub const CURRENT_3A_SET_MA: u32 = 3000;
pub const CURRENT_3A_CLEAR_MA: u32 = 2800;
pub const CURRENT_5A_SET_MA: u32 = 5000;
pub const CURRENT_5A_CLEAR_MA: u32 = 4800;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Tone {
    Boot,
    OperationOk,
    OperationDenied,
    ChannelPowerOn,
    ChannelPowerOff,
    HintCurrent3A,
    HintCurrent5A,
    HintInsert,
    HintRemove,
}

impl Tone {
    pub fn label(self) -> &'static str {
        match self {
            Self::Boot => "boot",
            Self::OperationOk => "operation_ok",
            Self::OperationDenied => "operation_denied",
            Self::ChannelPowerOn => "channel_power_on",
            Self::ChannelPowerOff => "channel_power_off",
            Self::HintCurrent3A => "hint_current_3a",
            Self::HintCurrent5A => "hint_current_5a",
            Self::HintInsert => "hint_insert",
            Self::HintRemove => "hint_remove",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AlarmTone {
    ChannelShort,
    OverTemp,
    InputOverPower,
    ChannelOver5A,
}

impl AlarmTone {
    pub fn label(self) -> &'static str {
        match self {
            Self::ChannelShort => "channel_short",
            Self::OverTemp => "over_temp",
            Self::InputOverPower => "input_over_power",
            Self::ChannelOver5A => "channel_over_5a",
        }
    }
}

pub fn choose_alarm(
    channel_short: bool,
    over_temp: bool,
    input_over_power: bool,
    channel_over_5a: bool,
) -> Option<AlarmTone> {
    if channel_short {
        Some(AlarmTone::ChannelShort)
    } else if over_temp {
        Some(AlarmTone::OverTemp)
    } else if input_over_power {
        Some(AlarmTone::InputOverPower)
    } else if channel_over_5a {
        Some(AlarmTone::ChannelOver5A)
    } else {
        None
    }
}

pub fn next_input_over_power(active: bool, vin_v: f32, i_a: f32) -> bool {
    let power_w = vin_v * i_a.abs();
    if active {
        power_w >= INPUT_OVER_POWER_CLEAR_W
    } else {
        power_w >= INPUT_OVER_POWER_SET_W
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct PortAudioTracker {
    inserted: bool,
    current_3a: bool,
    current_5a: bool,
}

impl PortAudioTracker {
    pub fn update(
        &mut self,
        vbus_mv: u32,
        current_ma: u32,
        suppress_normal_tone: bool,
    ) -> Option<Tone> {
        if suppress_normal_tone {
            self.synchronize(vbus_mv, current_ma);
            return None;
        }

        if !self.inserted {
            if vbus_mv >= PORT_INSERT_MV {
                self.inserted = true;
                self.current_3a = false;
                self.current_5a = false;
                return Some(Tone::HintInsert);
            }
            return None;
        }

        if vbus_mv < PORT_REMOVE_MV {
            self.inserted = false;
            self.current_3a = false;
            self.current_5a = false;
            return Some(Tone::HintRemove);
        }

        if current_ma < CURRENT_3A_CLEAR_MA {
            self.current_3a = false;
        }
        if current_ma < CURRENT_5A_CLEAR_MA {
            self.current_5a = false;
        }

        if !self.current_5a && current_ma >= CURRENT_5A_SET_MA {
            self.current_5a = true;
            self.current_3a = true;
            return Some(Tone::HintCurrent5A);
        }

        if !self.current_3a && current_ma >= CURRENT_3A_SET_MA {
            self.current_3a = true;
            return Some(Tone::HintCurrent3A);
        }

        None
    }

    fn synchronize(&mut self, vbus_mv: u32, current_ma: u32) {
        if vbus_mv >= PORT_INSERT_MV {
            self.inserted = true;
        } else if vbus_mv < PORT_REMOVE_MV {
            self.inserted = false;
        }

        if !self.inserted {
            self.current_3a = false;
            self.current_5a = false;
            return;
        }

        if current_ma >= CURRENT_5A_SET_MA {
            self.current_5a = true;
            self.current_3a = true;
        } else if current_ma < CURRENT_5A_CLEAR_MA {
            self.current_5a = false;
        }

        if current_ma >= CURRENT_3A_SET_MA {
            self.current_3a = true;
        } else if current_ma < CURRENT_3A_CLEAR_MA {
            self.current_3a = false;
        }
    }
}

pub fn center_button_tone(can_toggle: bool, next_enabled: bool) -> Tone {
    if !can_toggle {
        Tone::OperationDenied
    } else if next_enabled {
        Tone::ChannelPowerOn
    } else {
        Tone::ChannelPowerOff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alarm_priority_is_stable() {
        assert_eq!(
            choose_alarm(true, true, true, true),
            Some(AlarmTone::ChannelShort)
        );
        assert_eq!(
            choose_alarm(false, true, true, true),
            Some(AlarmTone::OverTemp)
        );
        assert_eq!(
            choose_alarm(false, false, true, true),
            Some(AlarmTone::InputOverPower)
        );
        assert_eq!(
            choose_alarm(false, false, false, true),
            Some(AlarmTone::ChannelOver5A)
        );
        assert_eq!(choose_alarm(false, false, false, false), None);
    }

    #[test]
    fn input_over_power_uses_hysteresis() {
        assert!(!next_input_over_power(false, 20.0, 4.9));
        assert!(next_input_over_power(false, 20.0, 5.0));
        assert!(next_input_over_power(true, 20.0, 4.5));
        assert!(!next_input_over_power(true, 20.0, 4.49));
    }

    #[test]
    fn port_tracker_reports_insert_remove_and_current_crossings_once() {
        let mut tracker = PortAudioTracker::default();

        assert_eq!(tracker.update(3299, 0, false), None);
        assert_eq!(tracker.update(3300, 0, false), Some(Tone::HintInsert));
        assert_eq!(tracker.update(5000, 3000, false), Some(Tone::HintCurrent3A));
        assert_eq!(tracker.update(5000, 3200, false), None);
        assert_eq!(tracker.update(5000, 5000, false), Some(Tone::HintCurrent5A));
        assert_eq!(tracker.update(5000, 4900, false), None);
        assert_eq!(tracker.update(5000, 4799, false), None);
        assert_eq!(tracker.update(5000, 5000, false), Some(Tone::HintCurrent5A));
        assert_eq!(tracker.update(2999, 0, false), Some(Tone::HintRemove));
    }

    #[test]
    fn protection_tick_suppresses_normal_hints() {
        let mut tracker = PortAudioTracker::default();

        assert_eq!(tracker.update(5000, 5200, true), None);
        assert_eq!(tracker.update(5000, 5200, false), None);
        assert_eq!(tracker.update(2999, 0, true), None);
        assert_eq!(tracker.update(2999, 0, false), None);
    }

    #[test]
    fn center_button_tone_reflects_acceptance_and_power_direction() {
        assert_eq!(center_button_tone(false, true), Tone::OperationDenied);
        assert_eq!(center_button_tone(true, true), Tone::ChannelPowerOn);
        assert_eq!(center_button_tone(true, false), Tone::ChannelPowerOff);
    }
}
