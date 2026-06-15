import type { ConnectionState } from "../../app/device-runtime";
import type { StoredDevice } from "../../domain/devices";
import {
  CANONICAL_PORT_IDS,
  type CanonicalPortId,
  type PortId,
  type PortState,
  type PortTelemetry,
} from "../../domain/ports";
import { formatTimeHms } from "../format/time";
import { PortMiniCard } from "./PortMiniCard";

export type DeviceSummaryCardProps = {
  device: StoredDevice;
  connection: {
    state: ConnectionState;
    lastOkAt?: number;
  };
  upstreamConnected: boolean | null;
  ports: Record<
    CanonicalPortId,
    { label: string; telemetry: PortTelemetry; state: PortState }
  >;
  onOpenDashboard: (deviceId: string) => void;
  onSetPower: (deviceId: string, portId: PortId, enabled: boolean) => void;
  onDataReplug: (deviceId: string, portId: PortId) => void;
};

function connectionBadge(state: ConnectionState): {
  bg: string;
  text: string;
  width: string;
} {
  if (state === "online") {
    return {
      bg: "bg-[var(--badge-success-bg)]",
      text: "text-[var(--badge-success-text)]",
      width: "w-[72px]",
    };
  }
  if (state === "offline") {
    return {
      bg: "bg-[var(--badge-error-bg)]",
      text: "text-[var(--badge-error-text)]",
      width: "w-[72px]",
    };
  }
  return {
    bg: "bg-[var(--badge-warning-bg)]",
    text: "text-[var(--badge-warning-text)]",
    width: "w-[96px]",
  };
}

export function DeviceSummaryCard({
  device,
  connection,
  upstreamConnected,
  ports,
  onOpenDashboard,
  onSetPower,
  onDataReplug,
}: DeviceSummaryCardProps) {
  const shortId = device.id.length > 8 ? device.id.slice(0, 8) : device.id;
  const lastOkLabel = connection.lastOkAt
    ? formatTimeHms(connection.lastOkAt)
    : "—";
  const upstreamLabel =
    upstreamConnected === null ? "—" : upstreamConnected ? "link" : "no link";
  const writeDisabled = connection.state !== "online";
  const badge = connectionBadge(connection.state);

  return (
    <div
      className="iso-card h-[272px] w-full rounded-[18px] bg-[var(--panel)] shadow-[inset_0_0_0_1px_var(--border)]"
      data-testid={`device-summary-${device.id}`}
    >
      <div className="flex h-full flex-col pb-[18px] pl-6 pr-6 pt-[14px]">
        <div className="h-[62px]">
          <div className="flex items-start justify-between gap-4">
            <div className="text-[16px] font-bold leading-5">{device.name}</div>
            <div
              className={[
                "flex h-6 items-center justify-center rounded-full",
                badge.width,
                badge.bg,
                badge.text,
                "text-[12px] font-semibold",
              ].join(" ")}
            >
              {connection.state}
            </div>
          </div>

          <div className="mt-3 font-mono text-[12px] font-semibold leading-[18px] text-[var(--muted)]">
            id: {shortId} • last ok: {lastOkLabel} • upstream: {upstreamLabel}
          </div>
        </div>

        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          {CANONICAL_PORT_IDS.map((portId) => (
            <PortMiniCard
              key={portId}
              portId={portId}
              label={ports[portId].label}
              telemetry={ports[portId].telemetry}
              state={ports[portId].state}
              disabled={writeDisabled}
              onSetPower={(enabled) => onSetPower(device.id, portId, enabled)}
              onReplug={() => onDataReplug(device.id, portId)}
            />
          ))}
        </div>

        <button
          className="mt-3 flex h-[34px] w-full flex-none items-center justify-center rounded-[10px] border border-[var(--border)] bg-transparent text-[12px] font-bold text-[var(--text)]"
          type="button"
          onClick={() => onOpenDashboard(device.id)}
        >
          Open dashboard →
        </button>
      </div>
    </div>
  );
}
