import { useMemo } from "react";
import { useNavigate } from "react-router";

import { useAddDeviceUi } from "../app/add-device-ui";
import { useDeviceRuntime } from "../app/device-runtime";
import { useDevices } from "../app/devices-store";
import {
  CANONICAL_PORT_IDS,
  type CanonicalPortId,
  type PortState,
  type PortTelemetry,
  portLabel,
} from "../domain/ports";
import { DeviceSummaryCard } from "../ui/cards/DeviceSummaryCard";

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

export function DashboardPage() {
  const navigate = useNavigate();
  const { openAddDevice } = useAddDeviceUi();
  const { devices } = useDevices();
  const runtime = useDeviceRuntime();

  const addDeviceCardSpan =
    devices.length % 2 === 0 ? "min-[1600px]:col-span-2" : "";

  const items = useMemo(
    () =>
      devices.map((d) => {
        const state = runtime.connectionState(d.id);
        const lastOkAt = runtime.lastOkAt(d.id) ?? undefined;
        const upstreamConnected =
          state === "online"
            ? (runtime.hub(d.id)?.upstream_connected ?? null)
            : null;

        const port = (portId: CanonicalPortId) => runtime.port(d.id, portId);
        const pending = (portId: CanonicalPortId) =>
          runtime.pending(d.id, portId);

        const ports = Object.fromEntries(
          CANONICAL_PORT_IDS.map((portId) => [
            portId,
            {
              label: portLabel(portId),
              telemetry:
                state === "online"
                  ? (port(portId)?.telemetry ?? fallbackTelemetry)
                  : fallbackTelemetry,
              state:
                state === "online"
                  ? mergedPortState(port(portId)?.state, pending(portId))
                  : fallbackState,
            },
          ]),
        ) as Record<
          CanonicalPortId,
          { label: string; telemetry: PortTelemetry; state: PortState }
        >;

        return {
          device: d,
          connection: { state, lastOkAt },
          upstreamConnected,
          ports,
        };
      }),
    [devices, runtime],
  );

  return (
    <div className="flex flex-col gap-5" data-testid="dashboard">
      <div className="iso-panel px-5 py-5 sm:px-6">
        <div className="iso-kicker">operator deck</div>
        <div className="mt-2 text-[30px] font-black leading-[0.94] tracking-[-0.05em]">
          Multi-device relay dashboard
        </div>
        <div className="mt-3 max-w-[72ch] text-[14px] font-medium leading-[1.6] text-[var(--muted)]">
          Scan live ports, compare power rails, and jump into per-device control
          without losing the measured state.
        </div>
        <div className="mt-4 flex flex-wrap gap-2">
          <span className="iso-chip iso-chip--signal">
            {devices.length} device{devices.length === 1 ? "" : "s"}
          </span>
          <span className="iso-chip">v / a / w telemetry</span>
          <span className="iso-chip">power + replug controls</span>
          <span className="iso-chip iso-chip--trace">proof before chrome</span>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-5 min-[1600px]:grid-cols-2">
        {items.map((item) => (
          <DeviceSummaryCard
            key={item.device.id}
            device={item.device}
            connection={item.connection}
            upstreamConnected={item.upstreamConnected}
            ports={item.ports}
            onOpenDashboard={(id) => navigate(`/devices/${id}`)}
            onSetPower={(deviceId, portId, enabled) =>
              void runtime.setPower(deviceId, portId, enabled)
            }
            onDataReplug={(deviceId, portId) =>
              void runtime.replug(deviceId, portId)
            }
          />
        ))}

        <button
          className={[
            "iso-panel flex min-h-[248px] w-full flex-col items-center justify-center",
            "border-dashed bg-[var(--add-placeholder-bg)] px-6 text-center",
            addDeviceCardSpan,
          ].join(" ")}
          type="button"
          onClick={openAddDevice}
          data-testid="dashboard-add-device-card"
        >
          <div className="iso-brand-mark" aria-hidden="true" />
          <div className="mt-5 text-[11px] font-extrabold uppercase tracking-[0.16em] text-[var(--primary)]">
            claim a new route
          </div>
          <div className="mt-3 text-[20px] font-black tracking-[-0.04em]">
            Add device
          </div>
          <div className="mt-2 max-w-[28ch] text-[12px] font-semibold leading-[1.55] text-[var(--muted)]">
            Create another hub entry and fold it into the same operator deck.
          </div>
        </button>
      </div>

      <div className="iso-panel-subtle px-4 py-3 text-[12px] font-semibold leading-[1.55] text-[var(--muted)]">
        Saved devices keep their last successful channel and fall back
        automatically when another path is available.
      </div>
    </div>
  );
}
