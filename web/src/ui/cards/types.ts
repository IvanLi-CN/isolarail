import type { PortId, PortState, PortTelemetry } from "../../domain/ports";

export type PortCardProps = {
  portId: PortId;
  label: string;
  telemetry: PortTelemetry;
  state: PortState;
  disabled?: boolean;
  onTogglePower: () => void;
  onReplug: () => void;
};
