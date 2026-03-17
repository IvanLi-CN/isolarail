use defmt::Format;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Format)]
#[repr(u8)]
pub enum BootStage {
    EarlyBringUp,
    SelfCheck,
    GateApply,
    Runtime,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Format)]
#[repr(u8)]
pub enum SelfCheckItemState {
    Pending,
    Ok,
    Warn,
    Err,
    Fatal,
    Skipped,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Format)]
#[repr(u8)]
pub enum BootOutcome {
    InProgress,
    Ok,
    Degraded,
    Fatal,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Format)]
pub enum BootFaultCode {
    None,
    PowerInUnavailable,
    PowerInPgBad,
    InaUnavailable,
    FrontPanelOffline,
    FanUnavailable,
    PortModuleOffline(u8),
    PortInaOffline(u8),
    PortTempOffline(u8),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SelfCheckSlot {
    pub state: SelfCheckItemState,
    pub fault: BootFaultCode,
}

impl SelfCheckSlot {
    pub const fn pending() -> Self {
        Self {
            state: SelfCheckItemState::Pending,
            fault: BootFaultCode::None,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct GateDecision {
    pub allow_runtime_tasks: bool,
    pub allow_front_panel: bool,
    pub allow_port: [bool; 4],
    pub keep_input_switch_open: bool,
    pub show_sticky_self_check: bool,
}

impl GateDecision {
    pub const fn new() -> Self {
        Self {
            allow_runtime_tasks: true,
            allow_front_panel: true,
            allow_port: [false; 4],
            keep_input_switch_open: false,
            show_sticky_self_check: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SysCheck {
    Vin,
    Mux,
    Front,
    Fan,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct BootSelfCheckSnapshot {
    pub stage: BootStage,
    pub outcome: BootOutcome,
    pub first_fault: BootFaultCode,
    pub gates: GateDecision,
    pub sys: [SelfCheckSlot; 4],
    pub ports: [SelfCheckSlot; 4],
}

impl BootSelfCheckSnapshot {
    pub const fn new() -> Self {
        Self {
            stage: BootStage::EarlyBringUp,
            outcome: BootOutcome::InProgress,
            first_fault: BootFaultCode::None,
            gates: GateDecision::new(),
            sys: [SelfCheckSlot::pending(); 4],
            ports: [SelfCheckSlot::pending(); 4],
        }
    }

    pub fn set_stage(&mut self, stage: BootStage) {
        self.stage = stage;
    }

    pub fn set_sys(&mut self, item: SysCheck, state: SelfCheckItemState, fault: BootFaultCode) {
        let idx = match item {
            SysCheck::Vin => 0,
            SysCheck::Mux => 1,
            SysCheck::Front => 2,
            SysCheck::Fan => 3,
        };
        self.sys[idx] = SelfCheckSlot { state, fault };
        self.latch_fault(fault);
    }

    pub fn set_port(&mut self, ch: usize, state: SelfCheckItemState, fault: BootFaultCode) {
        if ch >= self.ports.len() {
            return;
        }
        self.ports[ch] = SelfCheckSlot { state, fault };
        self.latch_fault(fault);
    }

    pub fn latch_fault(&mut self, fault: BootFaultCode) {
        if self.first_fault == BootFaultCode::None && fault != BootFaultCode::None {
            self.first_fault = fault;
        }
    }

    pub fn finalize(&mut self, gates: GateDecision) {
        self.gates = gates;
        self.outcome = if self
            .sys
            .iter()
            .chain(self.ports.iter())
            .any(|slot| slot.state == SelfCheckItemState::Fatal)
            || !gates.allow_runtime_tasks
        {
            BootOutcome::Fatal
        } else if self.sys.iter().chain(self.ports.iter()).any(|slot| {
            matches!(
                slot.state,
                SelfCheckItemState::Warn | SelfCheckItemState::Err
            )
        }) {
            BootOutcome::Degraded
        } else {
            BootOutcome::Ok
        };
    }
}

pub fn fault_label(fault: BootFaultCode) -> &'static str {
    match fault {
        BootFaultCode::None => "-",
        BootFaultCode::PowerInUnavailable => "VIN OFF",
        BootFaultCode::PowerInPgBad => "PG BAD",
        BootFaultCode::InaUnavailable => "INA OFF",
        BootFaultCode::FrontPanelOffline => "PANEL",
        BootFaultCode::FanUnavailable => "FAN",
        BootFaultCode::PortModuleOffline(_) => "MOD OFF",
        BootFaultCode::PortInaOffline(_) => "INA OFF",
        BootFaultCode::PortTempOffline(_) => "TMP OFF",
    }
}

pub fn state_label(state: SelfCheckItemState) -> &'static str {
    match state {
        SelfCheckItemState::Pending => "PEND",
        SelfCheckItemState::Ok => "OK",
        SelfCheckItemState::Warn => "WARN",
        SelfCheckItemState::Err => "ERR",
        SelfCheckItemState::Fatal => "FATAL",
        SelfCheckItemState::Skipped => "SKIP",
    }
}

pub fn outcome_label(outcome: BootOutcome) -> &'static str {
    match outcome {
        BootOutcome::InProgress => "CHECK",
        BootOutcome::Ok => "OK",
        BootOutcome::Degraded => "DEG",
        BootOutcome::Fatal => "FATAL",
    }
}
