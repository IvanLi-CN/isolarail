use core::fmt::{Result as FmtResult, Write};

use crate::boot_diag::{
    fault_label, outcome_label, state_label, BootSelfCheckSnapshot, BootStage, SelfCheckItemState,
};
use crate::{front_panel, hub_sideband};

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NodeState {
    Online,
    Offline,
    Skipped,
    Error,
}

impl NodeState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Skipped => "skipped",
            Self::Error => "error",
        }
    }

    pub const fn present(self) -> bool {
        matches!(self, Self::Online)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SensorProbe {
    pub present: bool,
    pub method: &'static str,
    pub tries: u8,
}

impl SensorProbe {
    pub const fn skipped() -> Self {
        Self {
            present: false,
            method: "skipped",
            tries: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PortProbe {
    pub ina226: SensorProbe,
    pub tmp112: SensorProbe,
}

impl PortProbe {
    pub const fn skipped() -> Self {
        Self {
            ina226: SensorProbe::skipped(),
            tmp112: SensorProbe::skipped(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PortRuntime {
    pub ui_state: &'static str,
    pub scan_done: bool,
    pub ready: bool,
    pub sample_ok: bool,
    pub manual_enabled: bool,
    pub pwren_enabled: bool,
    pub en_enabled: bool,
    pub ocp_latched: bool,
    pub vbus_mv: u32,
    pub current_ma: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PowerInputSnapshot {
    pub present: bool,
    pub state: SelfCheckItemState,
    pub fault: &'static str,
    pub vin_mv: i32,
    pub pg_good: bool,
    pub ready: bool,
    pub target_closed: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct I2cSnapshot {
    pub topology: &'static str,
    pub mux_state: NodeState,
    pub mux_address: u8,
    pub recovery_clocks: u8,
    pub reset_released_high: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct FanSnapshot {
    pub state: NodeState,
    pub ready: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn write_snapshot_json<const N: usize>(
    out: &mut heapless::String<N>,
    sequence: u32,
    uptime_ms: u64,
    reset_reason: &'static str,
    boot: &BootSelfCheckSnapshot,
    power: PowerInputSnapshot,
    i2c: I2cSnapshot,
    sideband: Option<hub_sideband::Snapshot>,
    sideband_state: NodeState,
    front: Option<front_panel::Snapshot>,
    front_state: NodeState,
    fan: FanSnapshot,
    ports: &[PortRuntime; 4],
    probes: &[PortProbe; 4],
    ina_addrs: &[u8; 4],
    tmp_addrs: &[u8; 4],
) -> FmtResult {
    out.clear();
    write!(
        out,
        "{{\"schema\":\"iso-usb-hub.hardware.snapshot.v{}\",\"sequence\":{},\"uptime_ms\":{},\"firmware\":{{\"name\":\"{}\",\"version\":\"{}\",\"target\":\"esp32s3\"}},\"reset_reason\":\"{}\",",
        SCHEMA_VERSION,
        sequence,
        uptime_ms,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        reset_reason
    )?;
    write_boot(out, boot)?;
    write_power(out, power)?;
    write_i2c(out, i2c)?;
    write_sideband(out, sideband, sideband_state)?;
    write_front_panel(out, front, front_state)?;
    write_fan(out, fan)?;
    write_ports(out, ports, probes, ina_addrs, tmp_addrs)?;
    write!(out, "}}")
}

fn write_boot<const N: usize>(
    out: &mut heapless::String<N>,
    boot: &BootSelfCheckSnapshot,
) -> FmtResult {
    write!(
        out,
        "\"boot\":{{\"stage\":\"{}\",\"outcome\":\"{}\",\"first_fault\":\"{}\",\"gates\":{{\"runtime\":{},\"front_panel\":{},\"keep_input_switch_open\":{},\"show_sticky_self_check\":{}}},\"checks\":{{",
        boot_stage_label(boot.stage),
        outcome_label(boot.outcome),
        fault_label(boot.first_fault),
        boot.gates.allow_runtime_tasks,
        boot.gates.allow_front_panel,
        boot.gates.keep_input_switch_open,
        boot.gates.show_sticky_self_check
    )?;
    let sys = ["vin", "mux", "front_panel", "fan"];
    for (idx, name) in sys.iter().enumerate() {
        if idx != 0 {
            write!(out, ",")?;
        }
        let slot = boot.sys[idx];
        write!(
            out,
            "\"{}\":{{\"state\":\"{}\",\"fault\":\"{}\"}}",
            name,
            state_label(slot.state),
            fault_label(slot.fault)
        )?;
    }
    write!(out, "}}}},")
}

fn boot_stage_label(stage: BootStage) -> &'static str {
    match stage {
        BootStage::EarlyBringUp => "early_bring_up",
        BootStage::SelfCheck => "self_check",
        BootStage::GateApply => "gate_apply",
        BootStage::Runtime => "runtime",
    }
}

fn write_power<const N: usize>(
    out: &mut heapless::String<N>,
    power: PowerInputSnapshot,
) -> FmtResult {
    write!(
        out,
        "\"power_input\":{{\"present\":{},\"state\":\"{}\",\"fault\":\"{}\",\"ina226\":{{\"address\":\"0x44\"}},\"vin_mv\":{},\"pg_good\":{},\"ready\":{},\"target\":\"{}\"}},",
        power.present,
        state_label(power.state),
        power.fault,
        power.vin_mv,
        power.pg_good,
        power.ready,
        if power.target_closed { "closed" } else { "open" }
    )
}

fn write_i2c<const N: usize>(out: &mut heapless::String<N>, i2c: I2cSnapshot) -> FmtResult {
    write!(
        out,
        "\"i2c\":{{\"topology\":\"{}\",\"mux\":{{\"present\":{},\"state\":\"{}\",\"address\":\"0x{:02X}\"}},\"recovery\":{{\"clocks\":{},\"reset_released_high\":{}}}}},",
        i2c.topology,
        i2c.mux_state.present(),
        i2c.mux_state.as_str(),
        i2c.mux_address,
        i2c.recovery_clocks,
        i2c.reset_released_high
    )
}

fn write_sideband<const N: usize>(
    out: &mut heapless::String<N>,
    sideband: Option<hub_sideband::Snapshot>,
    offline_state: NodeState,
) -> FmtResult {
    match sideband {
        Some(s) => write!(
            out,
            "\"sideband\":{{\"present\":true,\"state\":\"{}\",\"device\":\"TCA6408A\",\"address\":\"0x{:02X}\",\"registers\":{{\"input\":\"0x{:02X}\",\"output\":\"0x{:02X}\",\"polarity\":\"0x{:02X}\",\"config\":\"0x{:02X}\"}},\"pwren_enabled\":[{},{},{},{}],\"ovcur_asserted\":[{},{},{},{}]}},",
            offline_state.as_str(),
            hub_sideband::TCA6408_ADDR,
            s.input,
            s.output,
            s.polarity,
            s.config,
            s.pwren_enabled[0],
            s.pwren_enabled[1],
            s.pwren_enabled[2],
            s.pwren_enabled[3],
            s.ovcur_asserted[0],
            s.ovcur_asserted[1],
            s.ovcur_asserted[2],
            s.ovcur_asserted[3]
        ),
        None => write!(
            out,
            "\"sideband\":{{\"present\":false,\"state\":\"{}\",\"device\":\"TCA6408A\",\"address\":\"0x{:02X}\",\"reason\":\"{}\"}},",
            offline_state.as_str(),
            hub_sideband::TCA6408_ADDR,
            match offline_state {
                NodeState::Skipped => "vin_not_ready",
                NodeState::Error => "runtime_read_failed",
                _ => "no_ack_or_not_populated",
            }
        ),
    }
}

fn write_front_panel<const N: usize>(
    out: &mut heapless::String<N>,
    front: Option<front_panel::Snapshot>,
    requested_state: NodeState,
) -> FmtResult {
    match front {
        Some(s) => write!(
            out,
            "\"front_panel\":{{\"present\":true,\"state\":\"online\",\"device\":\"TCA6408A\",\"address\":\"0x21\",\"registers\":{{\"input\":\"0x{:02X}\",\"output\":\"0x{:02X}\",\"polarity\":\"0x{:02X}\",\"config\":\"0x{:02X}\"}},\"keys\":{{\"center\":{},\"right\":{},\"down\":{},\"left\":{},\"up\":{}}}}},",
            s.input,
            s.output,
            s.polarity,
            s.config,
            (s.input & (1 << 0)) == 0,
            (s.input & (1 << 1)) == 0,
            (s.input & (1 << 2)) == 0,
            (s.input & (1 << 3)) == 0,
            (s.input & (1 << 4)) == 0
        ),
        None => write!(
            out,
            "\"front_panel\":{{\"present\":false,\"state\":\"{}\",\"device\":\"TCA6408A\",\"address\":\"0x21\",\"reason\":\"{}\"}},",
            match requested_state {
                NodeState::Online => NodeState::Error.as_str(),
                other => other.as_str(),
            },
            match requested_state {
                NodeState::Skipped => "vin_not_ready",
                NodeState::Online | NodeState::Error => "runtime_read_failed",
                NodeState::Offline => "no_ack_or_not_populated",
            }
        ),
    }
}

fn write_fan<const N: usize>(out: &mut heapless::String<N>, fan: FanSnapshot) -> FmtResult {
    write!(
        out,
        "\"fan\":{{\"present\":{},\"state\":\"{}\",\"ready\":{}}},",
        fan.state.present(),
        fan.state.as_str(),
        fan.ready
    )
}

fn write_ports<const N: usize>(
    out: &mut heapless::String<N>,
    ports: &[PortRuntime; 4],
    probes: &[PortProbe; 4],
    ina_addrs: &[u8; 4],
    tmp_addrs: &[u8; 4],
) -> FmtResult {
    write!(out, "\"ports\":[")?;
    for idx in 0..4 {
        if idx != 0 {
            write!(out, ",")?;
        }
        let p = ports[idx];
        let probe = probes[idx];
        let state = if !p.scan_done || p.ui_state == "skipped" {
            NodeState::Skipped
        } else if p.ready {
            if p.sample_ok {
                NodeState::Online
            } else {
                NodeState::Error
            }
        } else if probe.ina226.present && probe.tmp112.present {
            NodeState::Online
        } else if probe.ina226.present || probe.tmp112.present {
            NodeState::Error
        } else {
            NodeState::Offline
        };
        write!(
            out,
            "{{\"index\":{},\"present\":{},\"state\":\"{}\",\"ready\":{},\"ui_state\":\"{}\",\"manual_enabled\":{},\"pwren_enabled\":{},\"en_enabled\":{},\"ocp_latched\":{},\"telemetry\":{{\"vbus_mv\":{},\"current_ma\":{}}},\"sensors\":{{\"ina226\":{{\"address\":\"0x{:02X}\",\"present\":{},\"method\":\"{}\",\"tries\":{}}},\"tmp112\":{{\"address\":\"0x{:02X}\",\"present\":{},\"method\":\"{}\",\"tries\":{}}}}}}}",
            idx + 1,
            state.present(),
            state.as_str(),
            p.ready,
            p.ui_state,
            p.manual_enabled,
            p.pwren_enabled,
            p.en_enabled,
            p.ocp_latched,
            p.vbus_mv,
            p.current_ma,
            ina_addrs[idx],
            probe.ina226.present,
            probe.ina226.method,
            probe.ina226.tries,
            tmp_addrs[idx],
            probe.tmp112.present,
            probe.tmp112.method,
            probe.tmp112.tries
        )?;
    }
    write!(out, "]")
}
