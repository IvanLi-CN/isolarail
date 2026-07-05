import { type ReactNode, useEffect, useMemo, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import { useDevices } from "../app/devices-store";
import {
  type HardwareSnapshot,
  type NodeState,
  mockSnapshot,
  summarize,
} from "../hardwareSnapshot";
import { DevicePageTabs } from "../ui/nav/DevicePageTabs";

type LoadState =
  | { kind: "loading"; snapshot: HardwareSnapshot }
  | { kind: "ready"; snapshot: HardwareSnapshot; source: "mock" | "devd" }
  | { kind: "error"; snapshot: HardwareSnapshot; message: string };

export function HardwareDebugPage() {
  const { deviceId } = useParams();
  const [searchParams] = useSearchParams();
  const { getDevice } = useDevices();
  const [loadState, setLoadState] = useState<LoadState>({
    kind: "ready",
    snapshot: mockSnapshot,
    source: "mock",
  });

  const devdBaseUrl = searchParams.get("devd");

  useEffect(() => {
    let cancelled = false;

    async function load() {
      if (!deviceId || !devdBaseUrl) {
        setLoadState({ kind: "ready", snapshot: mockSnapshot, source: "mock" });
        return;
      }
      setLoadState({ kind: "loading", snapshot: mockSnapshot });
      try {
        const base = devdBaseUrl.replace(/\/$/, "");
        const response = await fetch(
          `${base}/api/v1/devices/${encodeURIComponent(deviceId)}/diag-snapshot`,
        );
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }
        const snapshot = (await response.json()) as HardwareSnapshot;
        if (!cancelled) {
          setLoadState({ kind: "ready", snapshot, source: "devd" });
        }
      } catch (err) {
        if (!cancelled) {
          setLoadState({
            kind: "error",
            snapshot: mockSnapshot,
            message:
              err instanceof Error ? err.message : "Failed to load snapshot",
          });
        }
      }
    }

    void load();
    return () => {
      cancelled = true;
    };
  }, [deviceId, devdBaseUrl]);

  if (!deviceId) {
    return null;
  }

  const device = getDevice(deviceId);
  if (!device) {
    return (
      <div className="flex flex-col gap-3" data-testid="device-not-found">
        <div className="text-lg font-semibold">Device not found</div>
        <div className="text-sm opacity-80">
          Choose an existing device or add a new one.
        </div>
        <div>
          <Link className="link" to="/">
            Back to dashboard
          </Link>
        </div>
      </div>
    );
  }

  const snapshot = loadState.snapshot;
  const summary = summarize(snapshot);
  const shortId = device.id.length > 6 ? device.id.slice(0, 6) : device.id;
  const json = useMemo(() => JSON.stringify(snapshot, null, 2), [snapshot]);

  return (
    <div className="flex flex-col" data-testid="hardware-debug-page">
      <div>
        <div className="text-[24px] font-bold">{device.name}</div>
        <div className="mt-2 truncate font-mono text-[12px] font-semibold text-[var(--muted)]">
          hardware debug · id: {shortId} • {device.baseUrl}
        </div>
      </div>

      <div className="mt-4">
        <DevicePageTabs deviceId={deviceId} />
      </div>

      <div className="mt-[18px] grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_360px]">
        <div className="flex flex-col gap-4">
          <div className="iso-card rounded-[18px] bg-[var(--panel)] p-5 shadow-[inset_0_0_0_1px_var(--border)]">
            <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
              <div>
                <div className="text-[16px] font-bold">Hardware snapshot</div>
                <div className="mt-2 text-[12px] font-semibold leading-5 text-[var(--muted)]">
                  Read-only low-level snapshot for device bring-up and field
                  diagnostics.
                </div>
              </div>
              <StatusBadge state={loadState.kind === "error" ? "error" : "online"}>
                {loadState.kind === "loading"
                  ? "loading"
                  : loadState.kind === "ready"
                    ? loadState.source
                    : "mock fallback"}
              </StatusBadge>
            </div>
            {loadState.kind === "error" ? (
              <div className="mt-4 rounded-[12px] border border-[var(--warning)] bg-[var(--panel-2)] px-4 py-3 text-[12px] font-semibold text-[var(--warning)]">
                devd read failed: {loadState.message}
              </div>
            ) : null}
            <div className="mt-5 grid grid-cols-2 gap-3 md:grid-cols-4">
              <Metric label="Outcome" value={summary.outcome} />
              <Metric label="Ports online" value={String(summary.onlinePorts)} />
              <Metric label="Front panel" value={summary.frontPanel} />
              <Metric label="Sideband" value={summary.sideband} />
            </div>
          </div>

          <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
            <NodeCard
              title="Power input"
              state={snapshot.power_input.ready ? "online" : "error"}
              rows={[
                ["INA226", snapshot.power_input.ina226.address],
                ["VIN", `${snapshot.power_input.vin_mv} mV`],
                ["PG", String(snapshot.power_input.pg_good)],
                ["target", snapshot.power_input.target],
              ]}
            />
            <NodeCard
              title="I2C topology"
              state={snapshot.i2c.mux.state}
              rows={[
                ["topology", snapshot.i2c.topology],
                ["mux", snapshot.i2c.mux.address],
                ["clocks", String(snapshot.i2c.recovery.clocks)],
                [
                  "reset_high",
                  String(snapshot.i2c.recovery.reset_released_high),
                ],
              ]}
            />
            <NodeCard
              title="Sideband"
              state={snapshot.sideband.state}
              rows={[
                ["device", snapshot.sideband.device],
                ["address", snapshot.sideband.address],
                ["PWREN", (snapshot.sideband.pwren_enabled ?? []).join(", ")],
                ["OVCUR", (snapshot.sideband.ovcur_asserted ?? []).join(", ")],
              ]}
            />
            <NodeCard
              title="Front panel"
              state={snapshot.front_panel.state}
              rows={[
                ["device", snapshot.front_panel.device],
                ["address", snapshot.front_panel.address],
                ["reason", snapshot.front_panel.reason ?? "-"],
                ["keys", JSON.stringify(snapshot.front_panel.keys ?? {})],
              ]}
            />
          </div>

          <div className="iso-card rounded-[18px] bg-[var(--panel)] p-5 shadow-[inset_0_0_0_1px_var(--border)]">
            <div className="text-[16px] font-bold">Output modules</div>
            <div className="mt-4 overflow-hidden rounded-[14px] border border-[var(--border)]">
              <div className="grid grid-cols-[56px_92px_1fr_1fr] gap-3 border-b border-[var(--border)] bg-[var(--panel-2)] px-4 py-3 text-[11px] font-bold uppercase text-[var(--muted)]">
                <div>Port</div>
                <div>State</div>
                <div>Sensors</div>
                <div>Telemetry</div>
              </div>
              <div className="divide-y divide-[var(--border)]">
                {snapshot.ports.map((port) => (
                  <div
                    className="grid grid-cols-[56px_92px_1fr_1fr] gap-3 px-4 py-3 text-[12px] font-semibold"
                    key={port.index}
                  >
                    <div className="font-mono">P{port.index}</div>
                    <div>
                      <StatusBadge state={port.state}>{port.state}</StatusBadge>
                    </div>
                    <div className="min-w-0 font-mono text-[11px] text-[var(--muted)]">
                      INA {port.sensors.ina226.address}:{" "}
                      {String(port.sensors.ina226.present)}
                      <br />
                      TMP {port.sensors.tmp112.address}:{" "}
                      {String(port.sensors.tmp112.present)}
                    </div>
                    <div className="min-w-0 font-mono text-[11px]">
                      {port.telemetry.vbus_mv} mV /{" "}
                      {port.telemetry.current_ma} mA
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        <div className="iso-card h-fit rounded-[18px] bg-[var(--panel)] p-5 shadow-[inset_0_0_0_1px_var(--border)]">
          <div className="flex items-center justify-between gap-3">
            <div className="text-[16px] font-bold">JSON</div>
            <button
              className="btn btn-outline btn-sm min-h-9"
              type="button"
              onClick={() => void navigator.clipboard?.writeText(json)}
            >
              Copy
            </button>
          </div>
          <pre className="mt-4 max-h-[640px] overflow-auto rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] p-3 text-[11px] leading-5 text-[var(--muted)]">
            {json}
          </pre>
        </div>
      </div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-3 py-2">
      <div className="text-[11px] font-bold uppercase text-[var(--muted)]">
        {label}
      </div>
      <div className="mt-1 truncate font-mono text-[13px] font-semibold">
        {value}
      </div>
    </div>
  );
}

function NodeCard({
  title,
  state,
  rows,
}: {
  title: string;
  state: NodeState;
  rows: Array<[string, string]>;
}) {
  return (
    <div className="iso-card rounded-[18px] bg-[var(--panel)] p-5 shadow-[inset_0_0_0_1px_var(--border)]">
      <div className="flex items-center justify-between gap-3">
        <div className="text-[15px] font-bold">{title}</div>
        <StatusBadge state={state}>{state}</StatusBadge>
      </div>
      <div className="mt-4 flex flex-col gap-2">
        {rows.map(([label, value]) => (
          <div className="flex min-w-0 items-center text-[12px]" key={label}>
            <div className="w-[84px] shrink-0 font-semibold text-[var(--muted)]">
              {label}
            </div>
            <div className="min-w-0 truncate font-mono font-semibold">
              {value}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function StatusBadge({
  state,
  children,
}: {
  state: NodeState;
  children: ReactNode;
}) {
  const className =
    state === "online"
      ? "border-emerald-500/40 text-emerald-500"
      : state === "skipped"
        ? "border-sky-500/40 text-sky-500"
        : state === "offline"
          ? "border-[var(--border)] text-[var(--muted)]"
          : "border-[var(--warning)] text-[var(--warning)]";

  return (
    <span
      className={`inline-flex h-7 items-center rounded-[10px] border px-2 font-mono text-[11px] font-bold ${className}`}
    >
      {children}
    </span>
  );
}
