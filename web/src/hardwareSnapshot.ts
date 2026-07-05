export type NodeState = "online" | "offline" | "skipped" | "error";

export interface SensorProbe {
  address: string;
  present: boolean;
  method: string;
  tries: number;
}

export interface PortSnapshot {
  index: number;
  present: boolean;
  state: NodeState;
  ready: boolean;
  ui_state: string;
  manual_enabled: boolean;
  pwren_enabled: boolean;
  en_enabled: boolean;
  ocp_latched: boolean;
  telemetry: {
    vbus_mv: number;
    current_ma: number;
  };
  sensors: {
    ina226: SensorProbe;
    tmp112: SensorProbe;
  };
}

export interface HardwareSnapshot {
  schema: string;
  sequence: number;
  uptime_ms: number;
  firmware: {
    name: string;
    version: string;
    target: string;
  };
  reset_reason: string;
  boot: {
    stage: string;
    outcome: string;
    first_fault: string;
    gates: {
      runtime: boolean;
      front_panel: boolean;
      keep_input_switch_open: boolean;
      show_sticky_self_check: boolean;
    };
    checks: Record<string, { state: string; fault: string }>;
  };
  power_input: {
    present: boolean;
    state: string;
    fault: string;
    vin_mv: number;
    pg_good: boolean;
    ready: boolean;
    target: string;
    ina226: { address: string };
  };
  i2c: {
    topology: string;
    mux: { present: boolean; state: NodeState; address: string };
    recovery: { clocks: number; reset_released_high: boolean };
  };
  sideband: {
    present: boolean;
    state: NodeState;
    device: string;
    address: string;
    registers?: Record<string, string>;
    pwren_enabled?: boolean[];
    ovcur_asserted?: boolean[];
    reason?: string;
  };
  front_panel: {
    present: boolean;
    state: NodeState;
    device: string;
    address: string;
    registers?: Record<string, string>;
    keys?: Record<string, boolean>;
    reason?: string;
  };
  fan: { present: boolean; state: NodeState; ready: boolean };
  ports: PortSnapshot[];
}

export const mockSnapshot: HardwareSnapshot = {
  schema: "iso-usb-hub.hardware.snapshot.v1",
  sequence: 17,
  uptime_ms: 84231,
  firmware: { name: "iso-usb-hub", version: "0.1.0", target: "esp32s3" },
  reset_reason: "chip_power_on",
  boot: {
    stage: "runtime",
    outcome: "DEG",
    first_fault: "PANEL",
    gates: {
      runtime: true,
      front_panel: false,
      keep_input_switch_open: false,
      show_sticky_self_check: true,
    },
    checks: {
      vin: { state: "OK", fault: "-" },
      mux: { state: "SKIP", fault: "-" },
      front_panel: { state: "WARN", fault: "PANEL" },
      fan: { state: "OK", fault: "-" },
    },
  },
  power_input: {
    present: true,
    state: "OK",
    fault: "-",
    vin_mv: 12030,
    pg_good: true,
    ready: true,
    target: "closed",
    ina226: { address: "0x44" },
  },
  i2c: {
    topology: "direct_shared_bus",
    mux: { present: false, state: "skipped", address: "0x70" },
    recovery: { clocks: 18, reset_released_high: true },
  },
  sideband: {
    present: true,
    state: "online",
    device: "TCA6408A",
    address: "0x20",
    registers: { input: "0xAA", output: "0xFF", polarity: "0x00", config: "0xFF" },
    pwren_enabled: [true, true, false, true],
    ovcur_asserted: [false, false, false, false],
  },
  front_panel: {
    present: false,
    state: "offline",
    device: "TCA6408A",
    address: "0x21",
    reason: "no_ack_or_not_populated",
  },
  fan: { present: true, state: "online", ready: true },
  ports: [
    port(1, "online", true, "0x40", true, "0x48", true, 5010, 420),
    port(2, "error", false, "0x41", false, "0x49", true, 0, 0),
    port(3, "offline", false, "0x42", false, "0x4A", false, 0, 0),
    port(4, "online", true, "0x43", true, "0x4B", true, 4998, 810),
  ],
};

function port(
  index: number,
  state: NodeState,
  ready: boolean,
  inaAddress: string,
  inaPresent: boolean,
  tmpAddress: string,
  tmpPresent: boolean,
  vbusMv: number,
  currentMa: number,
): PortSnapshot {
  return {
    index,
    present: state === "online",
    state,
    ready,
    ui_state: ready ? "ok" : "disc",
    manual_enabled: true,
    pwren_enabled: index !== 3,
    en_enabled: ready,
    ocp_latched: false,
    telemetry: { vbus_mv: vbusMv, current_ma: currentMa },
    sensors: {
      ina226: { address: inaAddress, present: inaPresent, method: inaPresent ? "wr_rd" : "no", tries: inaPresent ? 1 : 6 },
      tmp112: { address: tmpAddress, present: tmpPresent, method: tmpPresent ? "wr_rd" : "no", tries: tmpPresent ? 1 : 6 },
    },
  };
}

export function deviceIdFromPath(pathname: string): string {
  const match = pathname.match(/^\/devices\/([^/]+)\/debug\/hardware\/?$/);
  return decodeURIComponent(match?.[1] ?? "local");
}

export function summarize(snapshot: HardwareSnapshot) {
  const onlinePorts = snapshot.ports.filter((portSnapshot) => portSnapshot.state === "online").length;
  const degradedPorts = snapshot.ports.length - onlinePorts;
  return {
    outcome: snapshot.boot.outcome,
    onlinePorts,
    degradedPorts,
    frontPanel: snapshot.front_panel.state,
    sideband: snapshot.sideband.state,
  };
}
