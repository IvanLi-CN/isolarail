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
pub struct Ina226Registers {
    pub config: u16,
    pub shunt_voltage: u16,
    pub bus_voltage: u16,
    pub power: u16,
    pub current: u16,
    pub calibration: u16,
    pub mask_enable: u16,
    pub alert_limit: u16,
    pub manufacturer_id: u16,
    pub die_id: u16,
}

impl Ina226Registers {
    pub const fn empty() -> Self {
        Self {
            config: 0,
            shunt_voltage: 0,
            bus_voltage: 0,
            power: 0,
            current: 0,
            calibration: 0,
            mask_enable: 0,
            alert_limit: 0,
            manufacturer_id: 0,
            die_id: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Ina226Runtime {
    pub state: NodeState,
    pub reason: &'static str,
    pub bus_voltage_mv: u32,
    pub shunt_voltage_uv: i32,
    pub current_ma: u32,
    pub registers: Ina226Registers,
}

impl Ina226Runtime {
    pub const fn skipped(reason: &'static str) -> Self {
        Self {
            state: NodeState::Skipped,
            reason,
            bus_voltage_mv: 0,
            shunt_voltage_uv: 0,
            current_ma: 0,
            registers: Ina226Registers::empty(),
        }
    }

    pub const fn offline(reason: &'static str) -> Self {
        Self {
            state: NodeState::Offline,
            reason,
            bus_voltage_mv: 0,
            shunt_voltage_uv: 0,
            current_ma: 0,
            registers: Ina226Registers::empty(),
        }
    }
}

impl Default for Ina226Runtime {
    fn default() -> Self {
        Self::skipped("runtime_not_sampled")
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Tmp112Registers {
    pub temperature: u16,
    pub config: u16,
    pub t_low: u16,
    pub t_high: u16,
}

impl Tmp112Registers {
    pub const fn empty() -> Self {
        Self {
            temperature: 0,
            config: 0,
            t_low: 0,
            t_high: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Tmp112Runtime {
    pub state: NodeState,
    pub reason: &'static str,
    pub temperature_milli_c: i32,
    pub registers: Tmp112Registers,
}

impl Tmp112Runtime {
    pub const fn skipped(reason: &'static str) -> Self {
        Self {
            state: NodeState::Skipped,
            reason,
            temperature_milli_c: 0,
            registers: Tmp112Registers::empty(),
        }
    }

    pub const fn offline(reason: &'static str) -> Self {
        Self {
            state: NodeState::Offline,
            reason,
            temperature_milli_c: 0,
            registers: Tmp112Registers::empty(),
        }
    }
}

impl Default for Tmp112Runtime {
    fn default() -> Self {
        Self::skipped("runtime_not_sampled")
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
    pub ina226: Ina226Runtime,
    pub tmp112: Tmp112Runtime,
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
    pub enabled: bool,
    pub tach_valid: bool,
    pub rpm: u32,
    pub target_rpm: u32,
    pub max_rpm: u32,
    pub speed_pct: u8,
    pub target_speed_pct: u8,
    pub hardware_pwm_duty_pct: u8,
    pub temperature_milli_c: i32,
    pub temperature_raw: u8,
    pub over_temp_alarm: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct McuSnapshot {
    pub state: NodeState,
    pub internal_temperature_milli_c: i32,
    pub internal_temperature_raw: u8,
    pub over_temp_alarm: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct BuzzerSnapshot {
    pub state: NodeState,
    pub driver_ready: bool,
    pub playing: bool,
    pub active_tone: &'static str,
    pub active_alarm: &'static str,
    pub frequency_hz: u16,
    pub duty_pct: u8,
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
    mcu: McuSnapshot,
    fan: FanSnapshot,
    buzzer: BuzzerSnapshot,
    ports: &[PortRuntime; 4],
    probes: &[PortProbe; 4],
    ina_addrs: &[u8; 4],
    tmp_addrs: &[u8; 4],
) -> FmtResult {
    out.clear();
    write!(
        out,
        "{{\"schema\":\"isolarail.hardware.snapshot.v{}\",\"packages\":[\"identity\",\"boot\",\"power\",\"i2c\",\"sideband\",\"front_panel\",\"mcu\",\"fan\",\"buzzer\",\"ports\",\"controls\",\"sensors\",\"registers\"],\"sequence\":{},\"uptime_ms\":{},\"firmware\":{{\"name\":\"{}\",\"version\":\"{}\",\"target\":\"esp32s3\"}},\"reset_reason\":\"{}\",",
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
    write_mcu(out, mcu)?;
    write_fan(out, fan)?;
    write_buzzer(out, buzzer)?;
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

fn write_mcu<const N: usize>(out: &mut heapless::String<N>, mcu: McuSnapshot) -> FmtResult {
    write!(
        out,
        "\"mcu\":{{\"present\":{},\"state\":\"{}\",\"internal_temperature\":{{\"milli_c\":{},\"raw\":{}}},\"over_temp_alarm\":{}}},",
        mcu.state.present(),
        mcu.state.as_str(),
        mcu.internal_temperature_milli_c,
        mcu.internal_temperature_raw,
        mcu.over_temp_alarm
    )
}

fn write_fan<const N: usize>(out: &mut heapless::String<N>, fan: FanSnapshot) -> FmtResult {
    write!(
        out,
        "\"fan\":{{\"present\":{},\"state\":\"{}\",\"ready\":{},\"enabled\":{},\"tach_valid\":{},\"rpm\":{},\"target_rpm\":{},\"max_rpm\":{},\"speed_pct\":{},\"target_speed_pct\":{},\"hardware_pwm_duty_pct\":{},\"temperature\":{{\"milli_c\":{},\"raw\":{}}},\"over_temp_alarm\":{}}},",
        fan.state.present(),
        fan.state.as_str(),
        fan.ready,
        fan.enabled,
        fan.tach_valid,
        fan.rpm,
        fan.target_rpm,
        fan.max_rpm,
        fan.speed_pct,
        fan.target_speed_pct,
        fan.hardware_pwm_duty_pct,
        fan.temperature_milli_c,
        fan.temperature_raw,
        fan.over_temp_alarm
    )
}

fn write_buzzer<const N: usize>(
    out: &mut heapless::String<N>,
    buzzer: BuzzerSnapshot,
) -> FmtResult {
    write!(
        out,
        "\"buzzer\":{{\"present\":{},\"state\":\"{}\",\"driver\":\"ledc\",\"timer\":1,\"channel\":1,\"gpio\":\"GPIO7\",\"driver_ready\":{},\"playing\":{},\"active_tone\":\"{}\",\"active_alarm\":\"{}\",\"frequency_hz\":{},\"duty_pct\":{}}},",
        buzzer.state.present(),
        buzzer.state.as_str(),
        buzzer.driver_ready,
        buzzer.playing,
        buzzer.active_tone,
        buzzer.active_alarm,
        buzzer.frequency_hz,
        buzzer.duty_pct
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
            "{{\"index\":{},\"present\":{},\"state\":\"{}\",\"ready\":{},\"ui_state\":\"{}\",\"manual_enabled\":{},\"pwren_enabled\":{},\"en_enabled\":{},\"ocp_latched\":{},\"control\":{{\"manual_enabled\":{},\"sideband_pwren_enabled\":{},\"module_en_enabled\":{},\"ocp_latched\":{},\"ready\":{},\"scan_done\":{}}},\"telemetry\":{{\"vbus_mv\":{},\"current_ma\":{}}},\"sensors\":{{",
            idx + 1,
            state.present(),
            state.as_str(),
            p.ready,
            p.ui_state,
            p.manual_enabled,
            p.pwren_enabled,
            p.en_enabled,
            p.ocp_latched,
            p.manual_enabled,
            p.pwren_enabled,
            p.en_enabled,
            p.ocp_latched,
            p.ready,
            p.scan_done,
            p.vbus_mv,
            p.current_ma
        )?;
        write_ina226_sensor(out, ina_addrs[idx], probe.ina226, p.ina226)?;
        write!(out, ",")?;
        write_tmp112_sensor(out, tmp_addrs[idx], probe.tmp112, p.tmp112)?;
        write!(out, "}}}}")?;
    }
    write!(out, "]")
}

fn write_ina226_sensor<const N: usize>(
    out: &mut heapless::String<N>,
    address: u8,
    probe: SensorProbe,
    runtime: Ina226Runtime,
) -> FmtResult {
    write!(
        out,
        "\"ina226\":{{\"address\":\"0x{:02X}\",\"present\":{},\"state\":\"{}\",\"method\":\"{}\",\"tries\":{},\"reason\":\"{}\"",
        address,
        probe.present,
        if probe.present {
            runtime.state.as_str()
        } else {
            NodeState::Offline.as_str()
        },
        probe.method,
        probe.tries,
        if probe.present {
            runtime.reason
        } else {
            "no_ack_or_not_populated"
        }
    )?;
    if probe.present && runtime.state == NodeState::Online {
        write!(
            out,
            ",\"reading\":{{\"bus_voltage_mv\":{},\"shunt_voltage_uv\":{},\"current_ma\":{}}},\"registers\":{{\"config\":\"0x{:04X}\",\"shunt_voltage\":\"0x{:04X}\",\"bus_voltage\":\"0x{:04X}\",\"power\":\"0x{:04X}\",\"current\":\"0x{:04X}\",\"calibration\":\"0x{:04X}\",\"mask_enable\":\"0x{:04X}\",\"alert_limit\":\"0x{:04X}\",\"manufacturer_id\":\"0x{:04X}\",\"die_id\":\"0x{:04X}\"}}",
            runtime.bus_voltage_mv,
            runtime.shunt_voltage_uv,
            runtime.current_ma,
            runtime.registers.config,
            runtime.registers.shunt_voltage,
            runtime.registers.bus_voltage,
            runtime.registers.power,
            runtime.registers.current,
            runtime.registers.calibration,
            runtime.registers.mask_enable,
            runtime.registers.alert_limit,
            runtime.registers.manufacturer_id,
            runtime.registers.die_id
        )?;
    }
    write!(out, "}}")
}

fn write_tmp112_sensor<const N: usize>(
    out: &mut heapless::String<N>,
    address: u8,
    probe: SensorProbe,
    runtime: Tmp112Runtime,
) -> FmtResult {
    write!(
        out,
        "\"tmp112\":{{\"address\":\"0x{:02X}\",\"present\":{},\"state\":\"{}\",\"method\":\"{}\",\"tries\":{},\"reason\":\"{}\"",
        address,
        probe.present,
        if probe.present {
            runtime.state.as_str()
        } else {
            NodeState::Offline.as_str()
        },
        probe.method,
        probe.tries,
        if probe.present {
            runtime.reason
        } else {
            "no_ack_or_not_populated"
        }
    )?;
    if probe.present && runtime.state == NodeState::Online {
        write!(
            out,
            ",\"reading\":{{\"temperature_milli_c\":{}}},\"registers\":{{\"temperature\":\"0x{:04X}\",\"config\":\"0x{:04X}\",\"t_low\":\"0x{:04X}\",\"t_high\":\"0x{:04X}\"}}",
            runtime.temperature_milli_c,
            runtime.registers.temperature,
            runtime.registers.config,
            runtime.registers.t_low,
            runtime.registers.t_high
        )?;
    }
    write!(out, "}}")
}
