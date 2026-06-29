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
    <div className="flex flex-col gap-6" data-testid="dashboard">
      <div>
        <div className="text-[24px] font-bold">Dashboard</div>
        <div className="mt-2 text-[14px] font-medium text-[var(--muted)]">
          Multi-device dashboard — V/A/W, status, quick actions
        </div>
      </div>

      <div className="grid grid-cols-1 gap-6 min-[1600px]:grid-cols-2">
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
            "iso-card flex min-h-[248px] w-full flex-col items-center justify-center",
            "rounded-[18px] border border-dashed border-[var(--border)]",
            "bg-[var(--add-placeholder-bg)] text-center",
            addDeviceCardSpan,
          ].join(" ")}
          type="button"
          onClick={openAddDevice}
          data-testid="dashboard-add-device-card"
        >
          <div className="text-[56px] font-extrabold text-[var(--muted)]">
            +
          </div>
          <div className="mt-4 text-[16px] font-bold">Add device</div>
          <div className="mt-2 text-[12px] font-semibold text-[var(--muted)]">
            Create a new hub entry
          </div>
        </button>
      </div>

      <div className="text-[12px] font-semibold text-[var(--muted)]">
        Saved devices keep their last successful channel and fall back
        automatically when another path is available.
      </div>
    </div>
  );
}
