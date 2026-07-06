export type NodeState = "online" | "offline" | "skipped" | "error";

export interface SensorProbe {
  address: string;
  present: boolean;
  state?: NodeState;
  method: string;
  tries: number;
  reason?: string;
  reading?: Record<string, number>;
  registers?: Record<string, string>;
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
  control?: {
    manual_enabled: boolean;
    sideband_pwren_enabled: boolean;
    module_en_enabled: boolean;
    ocp_latched: boolean;
    ready: boolean;
    scan_done: boolean;
  };
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
  packages?: string[];
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
  mcu?: {
    present: boolean;
    state: NodeState;
    internal_temperature: { milli_c: number; raw: number };
    over_temp_alarm: boolean;
  };
  fan: {
    present: boolean;
    state: NodeState;
    ready: boolean;
    enabled?: boolean;
    tach_valid?: boolean;
    rpm?: number;
    target_rpm?: number;
    max_rpm?: number;
    speed_pct?: number;
    target_speed_pct?: number;
    hardware_pwm_duty_pct?: number;
    temperature?: { milli_c: number; raw: number };
    over_temp_alarm?: boolean;
  };
  buzzer?: {
    present: boolean;
    state: NodeState;
    driver: string;
    timer: number;
    channel: number;
    gpio: string;
    driver_ready: boolean;
    playing: boolean;
    active_tone: string;
    active_alarm: string;
    frequency_hz: number;
    duty_pct: number;
  };
  ports: PortSnapshot[];
}

export const mockSnapshot: HardwareSnapshot = {
  schema: "iso-usb-hub.hardware.snapshot.v1",
  sequence: 17,
  uptime_ms: 84231,
  firmware: { name: "iso-usb-hub", version: "0.1.0", target: "esp32s3" },
  packages: [
    "identity",
    "boot",
    "power",
    "i2c",
    "sideband",
    "front_panel",
    "mcu",
    "fan",
    "buzzer",
    "ports",
    "controls",
    "sensors",
    "registers",
  ],
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
    registers: {
      input: "0xAA",
      output: "0xFF",
      polarity: "0x00",
      config: "0xFF",
    },
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
  mcu: {
    present: true,
    state: "online",
    internal_temperature: { milli_c: 44375, raw: 200 },
    over_temp_alarm: false,
  },
  fan: {
    present: true,
    state: "online",
    ready: true,
    enabled: true,
    tach_valid: true,
    rpm: 3120,
    target_rpm: 3000,
    max_rpm: 5200,
    speed_pct: 58,
    target_speed_pct: 55,
    hardware_pwm_duty_pct: 42,
    temperature: { milli_c: 44375, raw: 200 },
    over_temp_alarm: false,
  },
  buzzer: {
    present: true,
    state: "online",
    driver: "ledc",
    timer: 1,
    channel: 1,
    gpio: "GPIO7",
    driver_ready: true,
    playing: false,
    active_tone: "none",
    active_alarm: "none",
    frequency_hz: 0,
    duty_pct: 0,
  },
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
  const result: PortSnapshot = {
    index,
    present: state === "online",
    state,
    ready,
    ui_state: ready ? "ok" : "disc",
    manual_enabled: true,
    pwren_enabled: index !== 3,
    en_enabled: ready,
    ocp_latched: false,
    control: {
      manual_enabled: true,
      sideband_pwren_enabled: index !== 3,
      module_en_enabled: ready,
      ocp_latched: false,
      ready,
      scan_done: true,
    },
    telemetry: { vbus_mv: vbusMv, current_ma: currentMa },
    sensors: {
      ina226: {
        address: inaAddress,
        present: inaPresent,
        method: inaPresent ? "wr_rd" : "no",
        tries: inaPresent ? 1 : 6,
      },
      tmp112: {
        address: tmpAddress,
        present: tmpPresent,
        method: tmpPresent ? "wr_rd" : "no",
        tries: tmpPresent ? 1 : 6,
      },
    },
  };
  result.sensors.ina226.state = inaPresent ? state : "offline";
  result.sensors.ina226.reason = inaPresent ? "-" : "no_ack_or_not_populated";
  result.sensors.tmp112.state = tmpPresent ? state : "offline";
  result.sensors.tmp112.reason = tmpPresent ? "-" : "no_ack_or_not_populated";
  if (inaPresent) {
    result.sensors.ina226.reading = {
      bus_voltage_mv: vbusMv,
      shunt_voltage_uv: Math.round((currentMa / 1000) * 0.01 * 1_000_000),
      current_ma: currentMa,
    };
    result.sensors.ina226.registers = {
      config: "0x4127",
      shunt_voltage: "0x0004",
      bus_voltage: "0x0F98",
      power: "0x001A",
      current: "0x0000",
      calibration: "0x0000",
      mask_enable: "0x0000",
      alert_limit: "0x0000",
      manufacturer_id: "0x5449",
      die_id: "0x2260",
    };
  }
  if (tmpPresent) {
    result.sensors.tmp112.reading = { temperature_milli_c: 28625 };
    result.sensors.tmp112.registers = {
      temperature: "0x1CA0",
      config: "0x60A0",
      t_low: "0x4B00",
      t_high: "0x5000",
    };
  }
  return result;
}

export function deviceIdFromPath(pathname: string): string {
  const match = pathname.match(/^\/devices\/([^/]+)\/debug\/hardware\/?$/);
  return decodeURIComponent(match?.[1] ?? "local");
}

export function summarize(snapshot: HardwareSnapshot) {
  const onlinePorts = snapshot.ports.filter(
    (portSnapshot) => portSnapshot.state === "online",
  ).length;
  const degradedPorts = snapshot.ports.length - onlinePorts;
  return {
    outcome: snapshot.boot.outcome,
    onlinePorts,
    degradedPorts,
    frontPanel: snapshot.front_panel.state,
    sideband: snapshot.sideband.state,
  };
}
