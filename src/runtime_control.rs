use crate::device_contract::{WifiSnapshot, WifiState};
use crate::http_api_v1::ApiPendingAction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortControlAction {
    PowerSet { index: usize, enabled: bool },
    Replug { index: usize },
}

pub struct PortRuntimeState<'a> {
    pub manual_enabled: &'a mut [bool; 4],
    pub ocp_latched: &'a mut [bool; 4],
    pub ocp_safe_samples: &'a mut [u8; 4],
    pub ocp_retry_wait: &'a mut [u8; 4],
    pub replug_countdown: &'a mut [u8; 4],
}

impl From<ApiPendingAction> for PortControlAction {
    fn from(value: ApiPendingAction) -> Self {
        match value {
            ApiPendingAction::PortPower { index, enabled } => Self::PowerSet { index, enabled },
            ApiPendingAction::PortReplug { index } => Self::Replug { index },
        }
    }
}

pub fn tick_replug_countdowns(manual_enabled: &mut [bool; 4], replug_countdown: &mut [u8; 4]) {
    for idx in 0..4 {
        if replug_countdown[idx] > 0 {
            replug_countdown[idx] -= 1;
            if replug_countdown[idx] == 0 {
                manual_enabled[idx] = true;
            }
        }
    }
}

pub fn apply_port_action<F>(
    action: PortControlAction,
    state: &mut PortRuntimeState<'_>,
    replug_holdoff_ticks: u8,
    mut set_port_enable: F,
) -> bool
where
    F: FnMut(usize, bool),
{
    let index = match action {
        PortControlAction::PowerSet { index, .. } | PortControlAction::Replug { index } => index,
    };
    if index >= state.manual_enabled.len() {
        return false;
    }

    match action {
        PortControlAction::PowerSet { enabled, .. } => {
            state.manual_enabled[index] = enabled;
            if !enabled {
                state.ocp_latched[index] = false;
                state.ocp_safe_samples[index] = 0;
                state.ocp_retry_wait[index] = 0;
            }
            set_port_enable(index, enabled);
        }
        PortControlAction::Replug { .. } => {
            set_port_enable(index, false);
            state.manual_enabled[index] = false;
            state.replug_countdown[index] = replug_holdoff_ticks;
        }
    }
    true
}

pub fn apply_wifi_set_snapshot(wifi: &mut WifiSnapshot, ssid: &str, psk_configured: bool) -> bool {
    if ssid.is_empty() || ssid.len() > wifi.ssid.len() {
        return false;
    }

    wifi.configured = true;
    wifi.psk_configured = psk_configured;
    wifi.state = WifiState::Idle;
    wifi.ipv4 = None;
    wifi.is_static = false;
    wifi.ssid.fill(0);
    wifi.ssid[..ssid.len()].copy_from_slice(ssid.as_bytes());
    wifi.ssid_len = ssid.len() as u8;
    true
}

pub fn apply_wifi_clear_snapshot(wifi: &mut WifiSnapshot) {
    *wifi = WifiSnapshot::disconnected();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_wifi() -> WifiSnapshot {
        WifiSnapshot::disconnected()
    }

    #[test]
    fn api_pending_action_maps_to_runtime_port_action() {
        let action = PortControlAction::from(ApiPendingAction::PortPower {
            index: 2,
            enabled: true,
        });
        assert_eq!(
            action,
            PortControlAction::PowerSet {
                index: 2,
                enabled: true
            }
        );

        let action = PortControlAction::from(ApiPendingAction::PortReplug { index: 1 });
        assert_eq!(action, PortControlAction::Replug { index: 1 });
    }

    #[test]
    fn power_disable_clears_fault_state_and_drives_port() {
        let mut manual_enabled = [true; 4];
        let mut ocp_latched = [false; 4];
        let mut ocp_safe_samples = [0u8; 4];
        let mut ocp_retry_wait = [0u8; 4];
        let mut replug_countdown = [0u8; 4];
        let mut outputs = [true; 4];
        ocp_latched[1] = true;
        ocp_safe_samples[1] = 3;
        ocp_retry_wait[1] = 2;
        let mut state = PortRuntimeState {
            manual_enabled: &mut manual_enabled,
            ocp_latched: &mut ocp_latched,
            ocp_safe_samples: &mut ocp_safe_samples,
            ocp_retry_wait: &mut ocp_retry_wait,
            replug_countdown: &mut replug_countdown,
        };

        let applied = apply_port_action(
            PortControlAction::PowerSet {
                index: 1,
                enabled: false,
            },
            &mut state,
            2,
            |idx, enabled| outputs[idx] = enabled,
        );

        assert!(applied);
        assert!(!manual_enabled[1]);
        assert!(!ocp_latched[1]);
        assert_eq!(ocp_safe_samples[1], 0);
        assert_eq!(ocp_retry_wait[1], 0);
        assert!(!outputs[1]);
    }

    #[test]
    fn replug_disables_port_and_starts_holdoff() {
        let mut manual_enabled = [true; 4];
        let mut ocp_latched = [false; 4];
        let mut ocp_safe_samples = [0u8; 4];
        let mut ocp_retry_wait = [0u8; 4];
        let mut replug_countdown = [0u8; 4];
        let mut outputs = [true; 4];
        let mut state = PortRuntimeState {
            manual_enabled: &mut manual_enabled,
            ocp_latched: &mut ocp_latched,
            ocp_safe_samples: &mut ocp_safe_samples,
            ocp_retry_wait: &mut ocp_retry_wait,
            replug_countdown: &mut replug_countdown,
        };

        let applied = apply_port_action(
            PortControlAction::Replug { index: 3 },
            &mut state,
            2,
            |idx, enabled| outputs[idx] = enabled,
        );

        assert!(applied);
        assert!(!manual_enabled[3]);
        assert_eq!(replug_countdown[3], 2);
        assert!(!outputs[3]);
    }

    #[test]
    fn tick_replug_countdowns_restores_manual_enable_after_holdoff() {
        let mut manual_enabled = [true, false, true, true];
        let mut replug_countdown = [0, 1, 0, 0];

        tick_replug_countdowns(&mut manual_enabled, &mut replug_countdown);

        assert!(manual_enabled[1]);
        assert_eq!(replug_countdown[1], 0);
    }

    #[test]
    fn wifi_set_updates_runtime_snapshot() {
        let mut wifi = sample_wifi();

        let applied = apply_wifi_set_snapshot(&mut wifi, "Lab", true);

        assert!(applied);
        assert!(wifi.configured);
        assert!(wifi.psk_configured);
        assert_eq!(wifi.state, WifiState::Idle);
        assert_eq!(wifi.ssid(), Some("Lab"));
    }

    #[test]
    fn wifi_clear_resets_runtime_snapshot() {
        let mut wifi = sample_wifi();
        let _ = apply_wifi_set_snapshot(&mut wifi, "Lab", true);

        apply_wifi_clear_snapshot(&mut wifi);

        assert_eq!(wifi, WifiSnapshot::disconnected());
    }

    #[test]
    fn invalid_port_index_is_rejected_without_mutation() {
        let mut manual_enabled = [true; 4];
        let mut ocp_latched = [false; 4];
        let mut ocp_safe_samples = [0u8; 4];
        let mut ocp_retry_wait = [0u8; 4];
        let mut replug_countdown = [0u8; 4];
        let mut outputs = [true; 4];
        let mut state = PortRuntimeState {
            manual_enabled: &mut manual_enabled,
            ocp_latched: &mut ocp_latched,
            ocp_safe_samples: &mut ocp_safe_samples,
            ocp_retry_wait: &mut ocp_retry_wait,
            replug_countdown: &mut replug_countdown,
        };

        let applied = apply_port_action(
            PortControlAction::PowerSet {
                index: 8,
                enabled: false,
            },
            &mut state,
            2,
            |idx, enabled| outputs[idx] = enabled,
        );

        assert!(!applied);
        assert_eq!(manual_enabled, [true; 4]);
        assert_eq!(outputs, [true; 4]);
    }
}
