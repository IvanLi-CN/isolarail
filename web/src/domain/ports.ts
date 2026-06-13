export type CanonicalPortId = "port1" | "port2" | "port3" | "port4";
export type PortId = CanonicalPortId;

export const CANONICAL_PORT_IDS: CanonicalPortId[] = [
  "port1",
  "port2",
  "port3",
  "port4",
];

export function portLabel(portId: PortId): string {
  switch (portId) {
    case "port1":
      return "Port 1";
    case "port2":
      return "Port 2";
    case "port3":
      return "Port 3";
    case "port4":
      return "Port 4";
  }
}

export type TelemetryStatus = "ok" | "not_inserted" | "error" | "overrange";

export type HubState = {
  // Backward-compat: older firmware and summary cards use this field.
  upstream_connected: boolean;
  isolated_usb_fault?: boolean;
  isolated_downstream_connected?: boolean;
  isolated_usb_ready?: boolean;
};

export type PortTelemetry = {
  status: TelemetryStatus;
  voltage_mv: number | null;
  current_ma: number | null;
  power_mw: number | null;
  sample_uptime_ms: number;
};

export type PortState = {
  power_enabled: boolean;
  data_connected: boolean;
  replugging: boolean;
  busy: boolean;
  overcurrent?: boolean;
};

export type PortCapabilities = {
  data_replug: boolean;
  power_set: boolean;
};

export type Port = {
  portId: PortId;
  label: string;
  telemetry: PortTelemetry;
  state: PortState;
  capabilities: PortCapabilities;
};

export type PortsResponse = {
  // Backward-compat: older firmware may omit `hub` entirely.
  hub?: HubState;
  ports: Port[];
};
