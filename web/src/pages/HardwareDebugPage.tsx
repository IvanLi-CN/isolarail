import { Copy, Search, SkipBack, SkipForward } from "lucide-react";
import {
  type ReactNode,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { JsonView } from "react-json-view-lite";
import "react-json-view-lite/dist/index.css";
import { Link, useParams, useSearchParams } from "react-router";
import { useDevices } from "../app/devices-store";
import {
  type HardwareSnapshot,
  mockSnapshot,
  type NodeState,
  type SensorProbe,
  summarize,
} from "../hardwareSnapshot";
import { DevicePageTabs } from "../ui/nav/DevicePageTabs";

type LoadState =
  | { kind: "loading"; snapshot: HardwareSnapshot }
  | { kind: "ready"; snapshot: HardwareSnapshot; source: "mock" | "devd" }
  | { kind: "error"; snapshot: HardwareSnapshot; message: string };

type JsonPrimitive = string | number | boolean | null;
type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

function formatTemp(sensor: SensorProbe): string {
  const milliC = sensor.reading?.temperature_milli_c;
  if (typeof milliC !== "number") {
    return "temp --";
  }
  return `${(milliC / 1000).toFixed(2)} C`;
}

function formatMilliC(milliC: number | undefined): string {
  if (typeof milliC !== "number") {
    return "-- C";
  }
  return `${(milliC / 1000).toFixed(2)} C`;
}

function formatInaReading(sensor: SensorProbe): string {
  const busMv = sensor.reading?.bus_voltage_mv;
  const currentMa = sensor.reading?.current_ma;
  const shuntUv = sensor.reading?.shunt_voltage_uv;
  if (typeof busMv !== "number" || typeof currentMa !== "number") {
    return "reading --";
  }
  const shunt = typeof shuntUv === "number" ? ` · ${shuntUv} uV` : "";
  return `${busMv} mV · ${currentMa} mA${shunt}`;
}

function sensorState(sensor: SensorProbe): NodeState {
  return sensor.state ?? (sensor.present ? "online" : "offline");
}

function jsonPath(
  portIndex: number,
  sensor: "ina226" | "tmp112",
  leaf: string,
) {
  return `ports.${portIndex}.sensors.${sensor}.${leaf}`;
}

function ancestorsOf(path: string): string[] {
  const parts = path.split(".");
  const ancestors: string[] = [];
  for (let idx = 1; idx < parts.length; idx += 1) {
    ancestors.push(parts.slice(0, idx).join("."));
  }
  return ancestors;
}

function displayPath(path: string): string {
  if (!path) {
    return "$";
  }
  return `$.${path.replace(/\.(\d+)(?=\.|$)/g, "[$1]")}`;
}

function isJsonObject(value: JsonValue): value is { [key: string]: JsonValue } {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function jsonSummary(value: JsonValue): string {
  if (Array.isArray(value)) {
    return `Array(${value.length})`;
  }
  if (isJsonObject(value)) {
    return `{${Object.keys(value).length}}`;
  }
  if (typeof value === "string") {
    return `"${value}"`;
  }
  return String(value);
}

function valueMatches(
  path: string,
  keyName: string,
  value: JsonValue,
  query: string,
) {
  if (!query.trim()) {
    return false;
  }
  const needle = query.trim().toLowerCase();
  return (
    displayPath(path).toLowerCase().includes(needle) ||
    keyName.toLowerCase().includes(needle) ||
    jsonSummary(value).toLowerCase().includes(needle)
  );
}

function collectMatches(
  value: JsonValue,
  query: string,
  path = "",
  keyName = "$",
): string[] {
  const matches = valueMatches(path, keyName, value, query) ? [path] : [];
  if (Array.isArray(value)) {
    value.forEach((child, index) => {
      matches.push(
        ...collectMatches(
          child,
          query,
          path ? `${path}.${index}` : String(index),
          String(index),
        ),
      );
    });
  } else if (isJsonObject(value)) {
    for (const [key, child] of Object.entries(value)) {
      matches.push(
        ...collectMatches(child, query, path ? `${path}.${key}` : key, key),
      );
    }
  }
  return matches;
}

function buildObjectPathMap(value: JsonValue) {
  const paths = new WeakMap<object, string>();

  function visit(node: JsonValue, path: string) {
    if (typeof node !== "object" || node === null) {
      return;
    }
    paths.set(node, path);
    if (Array.isArray(node)) {
      node.forEach((child, index) => {
        visit(child, path ? `${path}.${index}` : String(index));
      });
      return;
    }
    for (const [key, child] of Object.entries(node)) {
      visit(child, path ? `${path}.${key}` : key);
    }
  }

  visit(value, "");
  return paths;
}

function directTreeItems(parent: Element) {
  return Array.from(parent.children).filter(
    (child): child is HTMLElement =>
      child instanceof HTMLElement && child.getAttribute("role") === "treeitem",
  );
}

function directGroup(parent: Element) {
  return Array.from(parent.children).find(
    (child): child is HTMLElement =>
      child instanceof HTMLElement && child.getAttribute("role") === "group",
  );
}

function annotateJsonTree(root: HTMLElement, value: JsonValue) {
  const tree = root.querySelector('[role="tree"]');
  if (!tree) {
    return;
  }

  function annotateItem(item: HTMLElement, node: JsonValue, path: string) {
    item.dataset.jsonPath = path;
    const group = directGroup(item);
    if (!group || typeof node !== "object" || node === null) {
      return;
    }
    const items = directTreeItems(group);
    if (Array.isArray(node)) {
      node.forEach((child, index) => {
        const childItem = items[index];
        if (childItem) {
          annotateItem(
            childItem,
            child,
            path ? `${path}.${index}` : `${index}`,
          );
        }
      });
      return;
    }
    Object.entries(node).forEach(([key, child], index) => {
      const childItem = items[index];
      if (childItem) {
        annotateItem(childItem, child, path ? `${path}.${key}` : key);
      }
    });
  }

  const [rootItem] = directTreeItems(tree);
  if (rootItem) {
    annotateItem(rootItem, value, "");
  }
}

function pathExists(value: JsonValue, path: string) {
  if (!path) {
    return true;
  }
  let current: JsonValue | undefined = value;
  for (const part of path.split(".")) {
    if (Array.isArray(current)) {
      const index = Number(part);
      if (!Number.isInteger(index) || index < 0 || index >= current.length) {
        return false;
      }
      current = current[index];
    } else if (isJsonObject(current)) {
      current = current[part];
    } else {
      return false;
    }
  }
  return current !== undefined;
}

const jsonViewerStyles = {
  container: "hardware-json-view",
  basicChildStyle: "hardware-json-node",
  childFieldsContainer: "hardware-json-children",
  label: "hardware-json-label",
  clickableLabel: "hardware-json-label hardware-json-label-clickable",
  nullValue: "hardware-json-null",
  undefinedValue: "hardware-json-null",
  numberValue: "hardware-json-number",
  stringValue: "hardware-json-string",
  booleanValue: "hardware-json-boolean",
  otherValue: "hardware-json-other",
  punctuation: "hardware-json-punctuation",
  expandIcon: "hardware-json-expander hardware-json-expander-collapsed",
  collapseIcon: "hardware-json-expander hardware-json-expander-expanded",
  collapsedContent: "hardware-json-collapsed",
  noQuotesForStringValues: false,
  quotesForFieldNames: false,
  stringifyStringValues: true,
};

export function HardwareDebugPage() {
  const { deviceId } = useParams();
  const [searchParams] = useSearchParams();
  const { getDevice } = useDevices();
  const [loadState, setLoadState] = useState<LoadState>({
    kind: "ready",
    snapshot: mockSnapshot,
    source: "mock",
  });

  const devdBaseUrl = searchParams.get("devd") ?? "/";
  const device = deviceId ? getDevice(deviceId) : undefined;
  const snapshotDeviceId = device?.transports?.localUsbDeviceId ?? deviceId;

  useEffect(() => {
    let cancelled = false;

    async function load() {
      if (!snapshotDeviceId || !devdBaseUrl) {
        setLoadState({ kind: "ready", snapshot: mockSnapshot, source: "mock" });
        return;
      }
      setLoadState({ kind: "loading", snapshot: mockSnapshot });
      try {
        const base = devdBaseUrl.replace(/\/$/, "");
        const bootstrap = await fetch(`${base}/api/v1/bootstrap`);
        if (!bootstrap.ok) {
          throw new Error(`HTTP ${bootstrap.status}`);
        }
        const bootstrapJson = (await bootstrap.json()) as { token?: string };
        const response = await fetch(
          `${base}/api/v1/devices/${encodeURIComponent(snapshotDeviceId)}/diag-snapshot`,
          {
            headers: bootstrapJson.token
              ? { Authorization: `Bearer ${bootstrapJson.token}` }
              : {},
          },
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
  }, [snapshotDeviceId, devdBaseUrl]);

  const snapshot = loadState.snapshot;
  const summary = summarize(snapshot);
  const activeDeviceId = device?.id ?? deviceId ?? "";
  const shortId =
    activeDeviceId.length > 6 ? activeDeviceId.slice(0, 6) : activeDeviceId;
  const [selectedJsonPath, setSelectedJsonPath] = useState("ports");
  const [expandedJsonPaths, setExpandedJsonPaths] = useState<Set<string>>(
    () =>
      new Set([
        "",
        "boot",
        "power_input",
        "sideband",
        "front_panel",
        "mcu",
        "fan",
        "buzzer",
        "ports",
        "ports.0",
        "ports.0.control",
        "ports.0.sensors",
        "ports.1",
        "ports.1.control",
        "ports.1.sensors",
      ]),
  );
  const json = useMemo(() => JSON.stringify(snapshot, null, 2), [snapshot]);
  const revealJsonPath = (path: string) => {
    setExpandedJsonPaths((current) => {
      const next = new Set(current);
      next.add(path);
      for (const ancestor of ancestorsOf(path)) {
        next.add(ancestor);
      }
      return next;
    });
    setSelectedJsonPath(path);
  };

  if (!deviceId) {
    return null;
  }

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

      <div className="mt-[18px] grid grid-cols-1 gap-4 2xl:grid-cols-[minmax(0,1fr)_minmax(460px,0.48fr)]">
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
              <StatusBadge
                state={loadState.kind === "error" ? "error" : "online"}
              >
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
              <Metric
                label="Ports online"
                value={String(summary.onlinePorts)}
              />
              <Metric label="Front panel" value={summary.frontPanel} />
              <Metric label="Sideband" value={summary.sideband} />
            </div>
          </div>

          <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
            <NodeCard
              title="MCU"
              state={snapshot.mcu?.state ?? "skipped"}
              selected={selectedJsonPath === "mcu"}
              inspectPath="mcu"
              onInspect={revealJsonPath}
              rows={[
                [
                  "temp",
                  formatMilliC(snapshot.mcu?.internal_temperature.milli_c),
                ],
                ["raw", String(snapshot.mcu?.internal_temperature.raw ?? "-")],
                ["alarm", String(snapshot.mcu?.over_temp_alarm ?? false)],
              ]}
            />
            <NodeCard
              title="Fan"
              state={snapshot.fan.state}
              selected={selectedJsonPath === "fan"}
              inspectPath="fan"
              onInspect={revealJsonPath}
              rows={[
                ["enabled", String(snapshot.fan.enabled ?? false)],
                [
                  "rpm",
                  `${snapshot.fan.rpm ?? 0} / ${snapshot.fan.target_rpm ?? 0}`,
                ],
                ["speed", `${snapshot.fan.speed_pct ?? 0}%`],
                ["hw_pwm", `${snapshot.fan.hardware_pwm_duty_pct ?? 0}%`],
              ]}
            />
            <NodeCard
              title="Buzzer"
              state={snapshot.buzzer?.state ?? "skipped"}
              selected={selectedJsonPath === "buzzer"}
              inspectPath="buzzer"
              onInspect={revealJsonPath}
              rows={[
                ["playing", String(snapshot.buzzer?.playing ?? false)],
                ["tone", snapshot.buzzer?.active_tone ?? "none"],
                ["alarm", snapshot.buzzer?.active_alarm ?? "none"],
                ["freq", `${snapshot.buzzer?.frequency_hz ?? 0} Hz`],
              ]}
            />
          </div>

          <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
            <NodeCard
              title="Power input"
              state={snapshot.power_input.ready ? "online" : "error"}
              selected={selectedJsonPath === "power_input"}
              inspectPath="power_input"
              onInspect={revealJsonPath}
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
              selected={selectedJsonPath === "i2c"}
              inspectPath="i2c"
              onInspect={revealJsonPath}
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
              selected={selectedJsonPath === "sideband"}
              inspectPath="sideband"
              onInspect={revealJsonPath}
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
              selected={selectedJsonPath === "front_panel"}
              inspectPath="front_panel"
              onInspect={revealJsonPath}
              rows={[
                ["device", snapshot.front_panel.device],
                ["address", snapshot.front_panel.address],
                ["reason", snapshot.front_panel.reason ?? "-"],
                ["keys", JSON.stringify(snapshot.front_panel.keys ?? {})],
              ]}
            />
          </div>

          <PortControlsCard
            ports={snapshot.ports}
            selectedPath={selectedJsonPath}
            onInspect={revealJsonPath}
          />

          <div className="iso-card rounded-[18px] bg-[var(--panel)] p-5 shadow-[inset_0_0_0_1px_var(--border)]">
            <div className="text-[16px] font-bold">Output modules</div>
            <div className="mt-4 overflow-hidden rounded-[14px] border border-[var(--border)]">
              <div className="grid grid-cols-[56px_92px_minmax(220px,1fr)_minmax(220px,1fr)] gap-3 border-b border-[var(--border)] bg-[var(--panel-2)] px-4 py-3 text-[11px] font-bold uppercase text-[var(--muted)]">
                <div>Port</div>
                <div>State</div>
                <div>INA226</div>
                <div>TMP112</div>
              </div>
              <div className="divide-y divide-[var(--border)]">
                {snapshot.ports.map((port) => (
                  <div
                    className="grid grid-cols-[56px_92px_minmax(220px,1fr)_minmax(220px,1fr)] gap-3 px-4 py-3 text-[12px] font-semibold"
                    key={port.index}
                  >
                    <div className="font-mono">P{port.index}</div>
                    <div>
                      <StatusBadge state={port.state}>{port.state}</StatusBadge>
                    </div>
                    <DeviceSensorCell
                      address={port.sensors.ina226.address}
                      label="INA"
                      pathBase={jsonPath(port.index - 1, "ina226", "")}
                      reading={formatInaReading(port.sensors.ina226)}
                      sensor={port.sensors.ina226}
                      onInspect={revealJsonPath}
                    />
                    <DeviceSensorCell
                      address={port.sensors.tmp112.address}
                      label="TMP"
                      pathBase={jsonPath(port.index - 1, "tmp112", "")}
                      reading={formatTemp(port.sensors.tmp112)}
                      sensor={port.sensors.tmp112}
                      onInspect={revealJsonPath}
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        <JsonInspector
          expandedPaths={expandedJsonPaths}
          jsonText={json}
          onExpandedPathsChange={setExpandedJsonPaths}
          onSelectedPathChange={setSelectedJsonPath}
          selectedPath={selectedJsonPath}
          value={snapshot as unknown as JsonValue}
        />
      </div>
    </div>
  );
}

function DeviceSensorCell({
  address,
  label,
  onInspect,
  pathBase,
  reading,
  sensor,
}: {
  address: string;
  label: string;
  onInspect: (path: string) => void;
  pathBase: string;
  reading: string;
  sensor: SensorProbe;
}) {
  const base = pathBase.replace(/\.$/, "");

  return (
    <div className="min-w-0 font-mono text-[11px] leading-5">
      <div className="flex min-w-0 flex-wrap items-baseline gap-x-2">
        <span className="font-bold text-[var(--ink)]">
          {label} {address}
        </span>
        <span className="text-[var(--muted)]">{sensorState(sensor)}</span>
      </div>
      <div className="truncate text-[var(--ink)]">{reading}</div>
      <div className="mt-1 flex flex-wrap gap-1.5">
        <InspectButton
          label="Registers"
          path={`${base}.registers`}
          onInspect={onInspect}
        />
        <InspectButton
          label="Reading"
          path={`${base}.reading`}
          onInspect={onInspect}
        />
      </div>
    </div>
  );
}

function PortControlsCard({
  onInspect,
  ports,
  selectedPath,
}: {
  onInspect: (path: string) => void;
  ports: HardwareSnapshot["ports"];
  selectedPath: string;
}) {
  const controls: Array<
    [string, (port: HardwareSnapshot["ports"][number]) => boolean]
  > = [
    ["Manual", (port) => port.control?.manual_enabled ?? port.manual_enabled],
    [
      "PWREN",
      (port) => port.control?.sideband_pwren_enabled ?? port.pwren_enabled,
    ],
    ["EN", (port) => port.control?.module_en_enabled ?? port.en_enabled],
    ["OCP", (port) => port.control?.ocp_latched ?? port.ocp_latched],
    ["Ready", (port) => port.control?.ready ?? port.ready],
    ["Scan", (port) => port.control?.scan_done ?? false],
  ];

  return (
    <div className="iso-card rounded-[18px] bg-[var(--panel)] p-5 shadow-[inset_0_0_0_1px_var(--border)]">
      <div className="flex items-center justify-between gap-3">
        <div className="text-[16px] font-bold">Port controls</div>
        <button
          className="rounded-[9px] border border-[var(--border)] bg-[var(--panel-2)] px-2.5 py-1 font-mono text-[10px] font-bold text-[var(--muted)] hover:border-[var(--primary)] hover:text-[var(--primary)]"
          type="button"
          onClick={() => onInspect("ports")}
        >
          ports
        </button>
      </div>
      <div className="mt-4 overflow-hidden rounded-[14px] border border-[var(--border)]">
        <div className="grid grid-cols-[72px_repeat(4,minmax(72px,1fr))] gap-2 border-b border-[var(--border)] bg-[var(--panel-2)] px-4 py-3 text-[11px] font-bold uppercase text-[var(--muted)]">
          <div>Gate</div>
          {ports.map((port) => (
            <button
              className="text-left hover:text-[var(--primary)]"
              key={port.index}
              type="button"
              onClick={() => onInspect(`ports.${port.index - 1}.control`)}
            >
              P{port.index}
            </button>
          ))}
        </div>
        <div className="divide-y divide-[var(--border)]">
          {controls.map(([label, read]) => (
            <div
              className="grid grid-cols-[72px_repeat(4,minmax(72px,1fr))] gap-2 px-4 py-2.5 text-[11px] font-semibold"
              key={label}
            >
              <div className="text-[var(--muted)]">{label}</div>
              {ports.map((port) => {
                const path = `ports.${port.index - 1}.control`;
                return (
                  <button
                    className={[
                      "min-w-0 rounded-[8px] border px-2 py-1 text-left font-mono transition",
                      selectedPath === path
                        ? "border-[var(--primary)] text-[var(--primary)]"
                        : "border-[var(--border)] text-[var(--ink)] hover:border-[var(--primary)] hover:text-[var(--primary)]",
                    ].join(" ")}
                    key={port.index}
                    type="button"
                    onClick={() => onInspect(path)}
                    title={displayPath(path)}
                  >
                    {String(read(port))}
                  </button>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function InspectButton({
  label,
  path,
  onInspect,
}: {
  label: string;
  path: string;
  onInspect: (path: string) => void;
}) {
  return (
    <button
      className="inline-flex h-6 items-center rounded-[8px] border border-[var(--border)] bg-[var(--panel-2)] px-2 font-mono text-[10px] font-bold text-[var(--muted)] transition hover:border-[var(--primary)] hover:text-[var(--primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]/30"
      type="button"
      onClick={() => onInspect(path)}
      title={displayPath(path)}
    >
      {label}
    </button>
  );
}

function JsonInspector({
  expandedPaths,
  jsonText,
  onExpandedPathsChange,
  onSelectedPathChange,
  selectedPath,
  value,
}: {
  expandedPaths: Set<string>;
  jsonText: string;
  onExpandedPathsChange: (next: Set<string>) => void;
  onSelectedPathChange: (path: string) => void;
  selectedPath: string;
  value: JsonValue;
}) {
  const [query, setQuery] = useState("");
  const [matchIndex, setMatchIndex] = useState(0);
  const treeRef = useRef<HTMLDivElement>(null);
  const objectPaths = useMemo(() => buildObjectPathMap(value), [value]);
  const matches = useMemo(() => collectMatches(value, query), [value, query]);
  const activeMatch =
    matches.length > 0 ? matches[matchIndex % matches.length] : "";
  const activePath = activeMatch || selectedPath;

  const shouldExpandNode = useCallback(
    (_level: number, node: unknown) => {
      if (typeof node !== "object" || node === null) {
        return false;
      }
      const path = objectPaths.get(node);
      return path !== undefined && expandedPaths.has(path);
    },
    [expandedPaths, objectPaths],
  );

  const beforeExpandChange = useCallback(
    ({
      newExpandValue,
      value: node,
    }: {
      newExpandValue: boolean;
      value: unknown;
    }) => {
      if (typeof node !== "object" || node === null) {
        return true;
      }
      const path = objectPaths.get(node);
      if (path === undefined) {
        return true;
      }
      const next = new Set(expandedPaths);
      if (newExpandValue) {
        next.add(path);
      } else {
        next.delete(path);
      }
      onExpandedPathsChange(next);
      return true;
    },
    [expandedPaths, objectPaths, onExpandedPathsChange],
  );

  useLayoutEffect(() => {
    const root = treeRef.current;
    if (!root) {
      return;
    }

    const applySelection = () => {
      annotateJsonTree(root, value);
      root.querySelectorAll("[data-json-selected]").forEach((node) => {
        if (node instanceof HTMLElement) {
          delete node.dataset.jsonSelected;
        }
      });
      if (!pathExists(value, activePath)) {
        return;
      }
      const selectedNode = Array.from(
        root.querySelectorAll("[data-json-path]"),
      ).find(
        (node): node is HTMLElement =>
          node instanceof HTMLElement && node.dataset.jsonPath === activePath,
      );
      if (selectedNode) {
        selectedNode.dataset.jsonSelected = "true";
        selectedNode.scrollIntoView({ block: "center", behavior: "smooth" });
      }
    };

    applySelection();
    const frame = requestAnimationFrame(applySelection);
    const timeout = window.setTimeout(applySelection, 80);
    return () => {
      cancelAnimationFrame(frame);
      window.clearTimeout(timeout);
    };
  });

  const revealPath = (path: string) => {
    onExpandedPathsChange(
      new Set([...expandedPaths, ...ancestorsOf(path), path]),
    );
    onSelectedPathChange(path);
  };

  const stepMatch = (direction: -1 | 1) => {
    if (matches.length === 0) {
      return;
    }
    const nextIndex =
      (matchIndex + direction + matches.length) % matches.length;
    setMatchIndex(nextIndex);
    revealPath(matches[nextIndex]);
  };

  return (
    <aside className="iso-card sticky top-4 h-fit rounded-[18px] bg-[var(--panel)] p-5 shadow-[inset_0_0_0_1px_var(--border)]">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="text-[16px] font-bold">JSON explorer</div>
          <div className="mt-1 max-w-[42ch] truncate font-mono text-[11px] font-semibold text-[var(--muted)]">
            {displayPath(selectedPath)}
          </div>
        </div>
        <button
          className="btn btn-outline btn-sm min-h-9 gap-2"
          type="button"
          onClick={() => void navigator.clipboard?.writeText(jsonText)}
        >
          <Copy size={14} />
          Copy
        </button>
      </div>

      <div className="mt-4 flex gap-2">
        <label className="flex h-9 min-w-0 flex-1 items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--panel-2)] px-3 text-[12px] font-semibold text-[var(--muted)] focus-within:border-[var(--primary)]">
          <Search size={14} />
          <input
            className="min-w-0 flex-1 bg-transparent font-mono text-[12px] text-[var(--text)] outline-none placeholder:text-[var(--muted)]"
            placeholder="Search path or value"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setMatchIndex(0);
            }}
          />
        </label>
        <button
          className="btn btn-outline btn-sm min-h-9 px-2"
          disabled={matches.length === 0}
          type="button"
          onClick={() => stepMatch(-1)}
          title="Previous match"
        >
          <SkipBack size={14} />
        </button>
        <button
          className="btn btn-outline btn-sm min-h-9 px-2"
          disabled={matches.length === 0}
          type="button"
          onClick={() => stepMatch(1)}
          title="Next match"
        >
          <SkipForward size={14} />
        </button>
      </div>
      <div className="mt-2 flex items-center justify-between font-mono text-[10px] font-bold text-[var(--muted)]">
        <span>{matches.length} matches</span>
        <button
          className="rounded-[8px] px-2 py-1 hover:bg-[var(--panel-2)] hover:text-[var(--primary)]"
          type="button"
          onClick={() => onExpandedPathsChange(new Set([""]))}
        >
          collapse all
        </button>
      </div>

      <div
        className="mt-4 max-h-[calc(100vh-220px)] min-h-[520px] overflow-auto rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] p-3 font-mono text-[11px] leading-5"
        ref={treeRef}
      >
        <JsonView
          aria-label="Hardware snapshot JSON"
          beforeExpandChange={beforeExpandChange}
          clickToExpandNode
          data={value as object}
          shouldExpandNode={shouldExpandNode}
          style={jsonViewerStyles}
        />
      </div>
    </aside>
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
  inspectPath,
  onInspect,
  selected,
  title,
  state,
  rows,
}: {
  inspectPath: string;
  onInspect: (path: string) => void;
  selected: boolean;
  title: string;
  state: NodeState;
  rows: Array<[string, string]>;
}) {
  return (
    <button
      className={[
        "iso-card rounded-[18px] bg-[var(--panel)] p-5 text-left shadow-[inset_0_0_0_1px_var(--border)] transition",
        "hover:shadow-[inset_0_0_0_1px_var(--primary)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]/30",
        selected ? "shadow-[inset_0_0_0_1px_var(--primary)]" : "",
      ].join(" ")}
      type="button"
      onClick={() => onInspect(inspectPath)}
      title={`Inspect ${displayPath(inspectPath)}`}
    >
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
    </button>
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
