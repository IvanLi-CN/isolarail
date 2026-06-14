import type { PortId, PortState, PortTelemetry } from "../../domain/ports";

export type PortCardProps = {
  portId: PortId;
  label: string;
  telemetry: PortTelemetry;
  state: PortState;
  disabled?: boolean;
  powerPending?: boolean;
  onTogglePower: () => void;
  onReplug: () => void;
};
