import { useMemo } from "react";
import { useDeviceRuntime } from "../../app/device-runtime";
import type { StoredDevice } from "../../domain/devices";
import {
  CANONICAL_PORT_IDS,
  type CanonicalPortId,
  type PortState,
  type PortTelemetry,
  portLabel,
} from "../../domain/ports";
import { PortCard } from "../cards/PortCard";
import { formatTimeHms } from "../format/time";

const fallbackTelemetry: PortTelemetry = {
  status: "error",
  voltage_mv: null,
  current_ma: null,
  power_mw: null,
  sample_uptime_ms: 0,
};

const fallbackState: PortState = {
  power_enabled: false,
  data_connected: false,
  replugging: false,
  busy: true,
  overcurrent: false,
};

function mergedPortState(
  state: PortState | undefined,
  pending: boolean,
): PortState {
  return {
    power_enabled: state?.power_enabled ?? false,
    data_connected: state?.data_connected ?? false,
    replugging: state?.replugging ?? false,
    busy: (state?.busy ?? true) || pending,
    overcurrent: state?.overcurrent ?? false,
  };
}

function statusBadge(state: "online" | "offline" | "unknown"): {
  tone: string;
  width: string;
} {
  if (state === "online") {
    return {
      tone: "iso-chip--success",
      width: "min-w-[96px]",
    };
  }
  if (state === "offline") {
    return {
      tone: "iso-chip--error",
      width: "min-w-[96px]",
    };
  }
  return {
    tone: "iso-chip--warning",
    width: "min-w-[96px]",
  };
}

function upstreamBadge(upstreamConnected: boolean | null): {
  tone: string;
  width: string;
  label: string;
} {
  if (upstreamConnected === null) {
    return {
      tone: "iso-chip--warning",
      width: "min-w-[96px]",
      label: "HOST —",
    };
  }
  if (upstreamConnected) {
    return {
      tone: "iso-chip--success",
      width: "min-w-[96px]",
      label: "HOST LINK",
    };
  }
  return {
    tone: "iso-chip--error",
    width: "min-w-[96px]",
    label: "NO HOST",
  };
}

function isolatedBadge(
  value: boolean | null,
  labels: { unknown: string; on: string; off: string },
): {
  tone: string;
  width: string;
  label: string;
} {
  if (value === null) {
    return {
      tone: "iso-chip--warning",
      width: "min-w-[112px]",
      label: labels.unknown,
    };
  }
  if (value) {
    return {
      tone: "iso-chip--success",
      width: "min-w-[112px]",
      label: labels.on,
    };
  }
  return {
    tone: "iso-chip--error",
    width: "min-w-[112px]",
    label: labels.off,
  };
}

function isolatedFaultBadge(value: boolean | null): {
  tone: string;
  width: string;
  label: string;
} {
  if (value === null) {
    return {
      tone: "iso-chip--warning",
      width: "min-w-[112px]",
      label: "ISO FAULT —",
    };
  }
  if (value) {
    return {
      tone: "iso-chip--error",
      width: "min-w-[112px]",
      label: "ISO FAULT",
    };
  }
  return {
    tone: "iso-chip--success",
    width: "min-w-[112px]",
    label: "ISO OK",
  };
}

function transportLabel(transport: "http" | "local_usb" | "web_serial" | null) {
  if (transport === "http") {
    return "Wi-Fi / LAN";
  }
  if (transport === "web_serial") {
    return "Web Serial";
  }
  if (transport === "local_usb") {
    return "Local USB";
  }
  return "—";
}

function shortChannelState(state: "online" | "offline" | "unknown"): string {
  if (state === "online") {
    return "on";
  }
  if (state === "offline") {
    return "off";
  }
  return "—";
}

export function DeviceDashboardPanel({ device }: { device: StoredDevice }) {
  const runtime = useDeviceRuntime();

  const connectionState = runtime.connectionState(device.id);
  const badge = statusBadge(connectionState);
  const hub = connectionState === "online" ? runtime.hub(device.id) : null;
  const upstream = upstreamBadge(
    connectionState === "online" ? (hub?.upstream_connected ?? null) : null,
  );
  const isolatedFault = isolatedFaultBadge(
    connectionState === "online" ? (hub?.isolated_usb_fault ?? null) : null,
  );
  const isolatedReady = isolatedBadge(
    connectionState === "online" ? (hub?.isolated_usb_ready ?? null) : null,
    {
      unknown: "ISO READY —",
      on: "ISO READY",
      off: "ISO WAIT",
    },
  );

  const lastOkAt = runtime.lastOkAt(device.id);
  const headerLastOk = lastOkAt === null ? "—" : formatTimeHms(lastOkAt);

  const rawBuildSha =
    (import.meta.env.VITE_BUILD_SHA as string | undefined) ?? "";
  const buildSha =
    rawBuildSha && rawBuildSha !== "dev" ? rawBuildSha.slice(0, 7) : "—";

  const transport = runtime.transport(device.id);
  const wifiState = runtime.channelState(device.id, "http");
  const webSerialState = runtime.channelState(device.id, "web_serial");
  const localUsbState = runtime.channelState(device.id, "local_usb");
  const notes =
    runtime.lastErrorLabel(device.id) ??
    `Primary: ${transportLabel(transport)} · Wi-Fi ${shortChannelState(wifiState)} · Web Serial ${shortChannelState(webSerialState)} · Local USB ${shortChannelState(localUsbState)}`;

  const writeDisabled =
    connectionState !== "online" || !runtime.usbWriteTransport(device.id);
  const headerBadges = [
    {
      key: "connection",
      label: connectionState.toUpperCase(),
      className: [badge.width, badge.tone, "text-[11px]"].join(" "),
    },
    {
      key: "upstream",
      label: upstream.label,
      className: [upstream.width, upstream.tone, "text-[11px]"].join(" "),
    },
    {
      key: "isolated-fault",
      label: isolatedFault.label,
      className: [isolatedFault.width, isolatedFault.tone, "text-[11px]"].join(
        " ",
      ),
    },
    {
      key: "isolated-ready",
      label: isolatedReady.label,
      className: [isolatedReady.width, isolatedReady.tone, "text-[11px]"].join(
        " ",
      ),
    },
  ];

  const items = useMemo(() => {
    const isOnline = connectionState === "online";

    const port = (portId: CanonicalPortId) => runtime.port(device.id, portId);
    const pending = (portId: CanonicalPortId) =>
      runtime.pending(device.id, portId);

    const telemetry = (portId: CanonicalPortId): PortTelemetry =>
      isOnline
        ? (port(portId)?.telemetry ?? fallbackTelemetry)
        : fallbackTelemetry;

    const state = (portId: CanonicalPortId): PortState =>
      isOnline
        ? mergedPortState(port(portId)?.state ?? undefined, pending(portId))
        : fallbackState;

    return Object.fromEntries(
      CANONICAL_PORT_IDS.map((portId) => [
        portId,
        {
          label: portLabel(portId),
          telemetry: telemetry(portId),
          state: state(portId),
          pending: isOnline ? pending(portId) : false,
        },
      ]),
    ) as Record<
      CanonicalPortId,
      {
        label: string;
        telemetry: PortTelemetry;
        state: PortState;
        pending: boolean;
      }
    >;
  }, [connectionState, device.id, runtime]);

  return (
    <div className="flex flex-col gap-5" data-testid="device-dashboard">
      <div className="iso-panel px-6 py-6">
        <div className="grid grid-cols-1 gap-5 leading-4 xl:grid-cols-[minmax(0,1fr)_minmax(320px,360px)] xl:items-start">
          <div className="grid min-w-0 gap-3 sm:grid-cols-[54px_minmax(0,1fr)] sm:items-start sm:gap-x-4">
            <div className="pt-[6px] text-[12px] font-semibold text-[var(--muted)] sm:pt-[4px]">
              Status
            </div>
            <div className="flex min-w-0 flex-wrap items-start gap-2">
              {headerBadges.map((item) => (
                <div
                  key={item.key}
                  className={[
                    "iso-chip h-7 shrink-0 items-center justify-center whitespace-nowrap px-3",
                    item.className,
                  ].join(" ")}
                >
                  {item.label}
                </div>
              ))}
            </div>
          </div>

          <div className="grid min-w-0 gap-y-3 sm:grid-cols-[72px_minmax(0,1fr)] sm:gap-x-4">
            <div className="text-[12px] font-semibold text-[var(--muted)]">
              Build
            </div>
            <div className="min-w-0 truncate font-mono text-[12px] font-semibold">
              {buildSha}
            </div>
            <div className="text-[12px] font-semibold text-[var(--muted)]">
              Last ok
            </div>
            <div className="font-mono text-[12px] font-semibold">
              {headerLastOk}
            </div>
            <div className="text-[12px] font-semibold text-[var(--muted)]">
              Notes
            </div>
            <div
              className="min-w-0 truncate text-[12px] font-semibold"
              title={notes}
            >
              {notes}
            </div>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 items-stretch gap-5 xl:grid-cols-2">
        {CANONICAL_PORT_IDS.map((portId) => (
          <PortCard
            key={portId}
            portId={portId}
            label={items[portId].label}
            telemetry={items[portId].telemetry}
            state={items[portId].state}
            disabled={writeDisabled}
            powerPending={items[portId].pending}
            onTogglePower={() =>
              void runtime.setPower(
                device.id,
                portId,
                !items[portId].state.power_enabled,
              )
            }
            onReplug={() => void runtime.replug(device.id, portId)}
          />
        ))}
      </div>
    </div>
  );
}
