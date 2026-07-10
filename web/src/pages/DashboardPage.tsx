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
  const hasDevices = devices.length > 0;

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
        <div className="text-[12px] font-semibold text-[var(--muted)]">
          Live overview
        </div>
        <div className="mt-2 text-[30px] font-black leading-[0.94] tracking-[-0.03em]">
          Multi-device relay dashboard
        </div>
        <div className="mt-3 max-w-[72ch] text-[14px] font-medium leading-[1.6] text-[var(--muted)]">
          Scan live ports, compare power rails, and jump into per-device control
          without losing the measured state.
        </div>
        {hasDevices ? (
          <div className="mt-4 flex flex-wrap gap-x-4 gap-y-2 text-[12px] font-semibold text-[var(--muted)]">
            <span>
              {devices.length} saved route{devices.length === 1 ? "" : "s"}
            </span>
            <span>Measured rail state</span>
            <span>Power and replug control</span>
          </div>
        ) : null}
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

        {hasDevices ? (
          <button
            className={[
              "iso-panel flex min-h-[248px] w-full flex-col items-center justify-center",
              "border-dashed bg-[var(--add-placeholder-bg)] px-6 text-center",
              addDeviceCardSpan,
            ].join(" ")}
            type="button"
            onClick={() => openAddDevice()}
            data-testid="dashboard-add-device-card"
          >
            <div className="iso-brand-mark" aria-hidden="true" />
            <div className="mt-5 text-[12px] font-semibold text-[var(--muted)]">
              Add another bench route
            </div>
            <div className="mt-3 text-[20px] font-black tracking-[-0.04em]">
              Add device
            </div>
            <div className="mt-2 max-w-[28ch] text-[12px] font-semibold leading-[1.55] text-[var(--muted)]">
              Create another hub entry and fold it into the same operator deck.
            </div>
          </button>
        ) : (
          <section
            className="iso-panel grid gap-5 bg-[var(--panel)] px-6 py-6 min-[1600px]:col-span-2 lg:grid-cols-[minmax(0,0.92fr)_minmax(0,1.08fr)]"
            data-testid="dashboard-add-device-card"
          >
            <div className="flex flex-col gap-6">
              <div>
                <div className="iso-brand-mark" aria-hidden="true" />
                <div className="mt-5 text-[12px] font-semibold text-[var(--muted)]">
                  First device setup
                </div>
                <div className="mt-3 text-[26px] font-black leading-[0.94] tracking-[-0.03em]">
                  Connect one device to start live rail control.
                </div>
                <div className="mt-4 max-w-[42ch] text-[13px] font-semibold leading-[1.7] text-[var(--muted)]">
                  Save the first hub identity, then keep discovery, power
                  control, replug actions, and measured rail state in the same
                  operator surface.
                </div>
              </div>
              <div className="grid gap-3 border-t border-[var(--border)] pt-4 sm:grid-cols-3">
                <div>
                  <div className="text-[12px] font-semibold text-[var(--muted)]">
                    Saved identity
                  </div>
                  <div className="mt-1 text-[13px] font-semibold leading-[1.55] text-[var(--text)]">
                    One claimed bench record with the last known working route.
                  </div>
                </div>
                <div>
                  <div className="text-[12px] font-semibold text-[var(--muted)]">
                    Fallback path
                  </div>
                  <div className="mt-1 text-[13px] font-semibold leading-[1.55] text-[var(--text)]">
                    Local USB can recover setup before Wi-Fi or Web Serial is
                    stable.
                  </div>
                </div>
                <div>
                  <div className="text-[12px] font-semibold text-[var(--muted)]">
                    Operator view
                  </div>
                  <div className="mt-1 text-[13px] font-semibold leading-[1.55] text-[var(--text)]">
                    Power control, replug, and measured rails stay in one
                    surface.
                  </div>
                </div>
              </div>
              <div className="flex flex-wrap items-center gap-3">
                <button
                  className="iso-button iso-button--primary"
                  type="button"
                  onClick={() => openAddDevice()}
                >
                  Add first device
                </button>
                <div className="text-[12px] font-semibold text-[var(--muted)]">
                  Choose a route below if you already know the attachment path.
                </div>
              </div>
            </div>

            <div className="rounded-[18px] border border-[var(--border)] bg-[var(--panel-2)] px-4 py-4">
              <div className="text-[13px] font-semibold text-[var(--muted)]">
                Start paths
              </div>
              <div className="mt-4 grid gap-3">
                <button
                  className="rounded-[14px] border border-[var(--border)] bg-[var(--panel)] px-4 py-4 text-left transition-colors hover:border-[var(--primary)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[color-mix(in_srgb,var(--primary)_72%,var(--trace))] focus-visible:outline-offset-2"
                  type="button"
                  onClick={() => openAddDevice("local_usb")}
                >
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="text-[15px] font-bold">Local USB</div>
                    <div className="text-[12px] font-semibold text-[var(--primary)]">
                      Best for bring-up
                    </div>
                  </div>
                  <div className="mt-2 text-[12px] font-semibold leading-[1.6] text-[var(--muted)]">
                    Use the companion-backed USB path when you need stable
                    identity, direct bench proof, or recovery from an unknown
                    network state.
                  </div>
                </button>
                <button
                  className="rounded-[14px] border border-[var(--border)] bg-[var(--panel)] px-4 py-4 text-left transition-colors hover:border-[var(--primary)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[color-mix(in_srgb,var(--primary)_72%,var(--trace))] focus-visible:outline-offset-2"
                  type="button"
                  onClick={() => openAddDevice("web_serial")}
                >
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="text-[15px] font-bold">Web Serial</div>
                    <div className="text-[12px] font-semibold text-[var(--muted)]">
                      Browser attach
                    </div>
                  </div>
                  <div className="mt-2 text-[12px] font-semibold leading-[1.6] text-[var(--muted)]">
                    Use the browser picker for a quick first inspection when the
                    hub is on the desk but not yet saved into the app.
                  </div>
                </button>
                <button
                  className="rounded-[14px] border border-[var(--border)] bg-[var(--panel)] px-4 py-4 text-left transition-colors hover:border-[var(--primary)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[color-mix(in_srgb,var(--primary)_72%,var(--trace))] focus-visible:outline-offset-2"
                  type="button"
                  onClick={() => openAddDevice("wifi")}
                >
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="text-[15px] font-bold">Wi-Fi / LAN</div>
                    <div className="text-[12px] font-semibold text-[var(--muted)]">
                      Steady sessions
                    </div>
                  </div>
                  <div className="mt-2 text-[12px] font-semibold leading-[1.6] text-[var(--muted)]">
                    Use the network route once the companion or browser can
                    already see the hub and you want a stable long-session
                    dashboard path.
                  </div>
                </button>
              </div>
            </div>
          </section>
        )}
      </div>

      <div className="iso-panel-subtle px-4 py-3 text-[12px] font-semibold leading-[1.55] text-[var(--muted)]">
        Saved devices keep their last successful channel and fall back
        automatically when another path is available.
      </div>
    </div>
  );
}
