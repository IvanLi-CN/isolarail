import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import {
  type CompanionBridge,
  tryBootstrapCompanionBridge,
} from "../domain/companionBridge";
import {
  clearWifiConfig,
  type DeviceApiError,
  type DeviceInfoResponse,
  getDeviceInfo,
  getPorts,
  getWifiConfig,
  type RebootResponse,
  type Result,
  rebootDevice,
  replugPort,
  setPortPower,
  setWifiConfig,
  type WifiConfigInput,
  type WifiConfigResponse,
  type WifiMutationResponse,
} from "../domain/deviceApi";
import {
  nextJsonlRequestId,
  sendDevdLocalUsbJsonlRequest,
  sendLocalUsbJsonlRequest,
} from "../domain/hardwareConsole";
import {
  getLocalUsbDeviceLink,
  subscribeLocalUsbDeviceLinks,
} from "../domain/localUsbLinks";
import {
  announceNetworkDeviceLink,
  subscribeNetworkDeviceLinks,
} from "../domain/networkLinks";
import type {
  CanonicalPortId,
  HubState,
  Port,
  PortId,
  PortsResponse,
} from "../domain/ports";
import { CANONICAL_PORT_IDS, portLabel } from "../domain/ports";
import {
  forgetWebSerialDeviceTransport,
  getWebSerialDeviceTransport,
  subscribeWebSerialDeviceLinks,
} from "../domain/webSerialLinks";
import { useToast } from "../ui/toast/ToastProvider";
import {
  type ConnectionState,
  createEmptyChannels,
  type DeviceRuntime,
  type DeviceRuntimeContextValue,
  type DeviceTransport,
  emptyPendingPorts,
  httpBaseUrlForDevice,
  isDeviceInfoResponse,
  localUsbDeviceIdForDevice,
  localUsbErrorToDeviceApiError,
  shortApiError,
  shouldResetLocalUsbConnectionCache,
  uniqueTransports,
} from "./device-runtime-support";
import { useDevices } from "./devices-store";

export type {
  ConnectionState,
  DeviceTransport,
} from "./device-runtime-support";
export {
  localUsbErrorToDeviceApiError,
  shouldResetLocalUsbConnectionCache,
} from "./device-runtime-support";

const DeviceRuntimeContext = createContext<DeviceRuntimeContextValue | null>(
  null,
);

const OFFLINE_THRESHOLD_MS = 10_000;
const POWER_ECHO_ATTEMPTS = 12;
const POWER_ECHO_DELAY_MS = 150;

function announceWifiHttpLink(
  deviceId: string,
  wifi: WifiConfigResponse | WifiMutationResponse,
): boolean {
  const ipv4 = wifi.ipv4?.trim();
  if (wifi.state !== "connected" || !ipv4) {
    return false;
  }
  announceNetworkDeviceLink({ deviceId, baseUrl: `http://${ipv4}` });
  return true;
}

type JsonlEnvelope<T> = {
  id?: number | string | null;
  ok: boolean;
  result?: T;
  error?: { code: string; message: string; retryable: boolean };
};

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    globalThis.setTimeout(resolve, ms);
  });
}

function normalizeHubSnapshot(hub: HubState | null): HubState | null {
  if (!hub) {
    return null;
  }
  return {
    ...hub,
    isolated_downstream_connected:
      hub.isolated_downstream_connected ?? hub.upstream_connected,
    isolated_usb_ready: hub.isolated_usb_ready ?? hub.upstream_connected,
  };
}

function normalizePortsResponse(
  response: PortsResponse,
): Record<CanonicalPortId, Port> | null {
  const normalized = new Map<CanonicalPortId, Port>();
  for (const port of response.ports) {
    if (normalized.has(port.portId)) {
      continue;
    }
    normalized.set(port.portId, {
      ...port,
      label: port.label.trim() || portLabel(port.portId),
    });
  }
  for (const portId of CANONICAL_PORT_IDS) {
    if (!normalized.has(portId)) {
      return null;
    }
  }
  const orderedPorts: Partial<Record<CanonicalPortId, Port>> = {};
  for (const portId of CANONICAL_PORT_IDS) {
    const port = normalized.get(portId);
    if (!port) {
      return null;
    }
    orderedPorts[portId] = port;
  }
  return orderedPorts as Record<CanonicalPortId, Port>;
}

export function DeviceRuntimeProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const { devices, persistDevice, refreshDevices } = useDevices();
  const { pushToast } = useToast();
  const [now, setNow] = useState(() => Date.now());
  const [runtimeById, setRuntimeById] = useState<Record<string, DeviceRuntime>>(
    {},
  );
  const runtimeByIdRef = useRef(runtimeById);
  const inflight = useRef<Set<string>>(new Set());
  const localUsbAgent = useRef<CompanionBridge | null>(null);
  const localUsbPortByDevice = useRef<Record<string, string>>({});
  const localUsbRequestQueues = useRef<Record<string, Promise<void>>>({});
  const preferredTransportByDevice = useRef<Record<string, DeviceTransport>>(
    {},
  );

  useEffect(() => {
    runtimeByIdRef.current = runtimeById;
  }, [runtimeById]);

  useEffect(() => {
    setRuntimeById((prev) => {
      const next: Record<string, DeviceRuntime> = { ...prev };
      const alive = new Set(devices.map((d) => d.id));
      for (const id of Object.keys(next)) {
        if (!alive.has(id)) {
          delete next[id];
          delete localUsbPortByDevice.current[id];
          delete localUsbRequestQueues.current[id];
          delete preferredTransportByDevice.current[id];
        }
      }
      for (const d of devices) {
        if (!next[d.id]) {
          next[d.id] = {
            lastOkAt: null,
            lastError: null,
            transport: null,
            channels: createEmptyChannels(),
            hub: null,
            ports: null,
            pending: emptyPendingPorts(),
          };
        }
      }
      return next;
    });
  }, [devices]);

  const getLocalUsbAgent =
    useCallback(async (): Promise<CompanionBridge | null> => {
      if (localUsbAgent.current) {
        return localUsbAgent.current;
      }
      const agent = await tryBootstrapCompanionBridge();
      localUsbAgent.current = agent;
      return agent;
    }, []);

  const findLocalUsbTarget = useCallback(
    async (
      deviceId: string,
    ): Promise<
      | { kind: "port_path"; portPath: string }
      | { kind: "devd_device"; deviceId: string }
      | null
    > => {
      const cached = localUsbPortByDevice.current[deviceId];
      if (cached) {
        return { kind: "port_path", portPath: cached };
      }
      const linked = getLocalUsbDeviceLink(deviceId);
      if (linked) {
        localUsbPortByDevice.current[deviceId] = linked;
        return { kind: "port_path", portPath: linked };
      }
      const stored = devices.find((device) => device.id === deviceId);
      const devdDeviceId = stored ? localUsbDeviceIdForDevice(stored) : null;
      if (devdDeviceId) {
        return { kind: "devd_device", deviceId: devdDeviceId };
      }
      return null;
    },
    [devices],
  );

  const requestLocalUsb = useCallback(
    async <T,>(
      deviceId: string,
      method: string,
      params?: Record<string, unknown>,
    ): Promise<Result<T>> => {
      const agent = await getLocalUsbAgent();
      if (!agent) {
        return {
          ok: false,
          error: { kind: "offline", message: "Local companion unavailable" },
        };
      }
      const target = await findLocalUsbTarget(deviceId);
      if (!target) {
        return {
          ok: false,
          error: { kind: "offline", message: "Local USB device not found" },
        };
      }
      const previous =
        localUsbRequestQueues.current[deviceId] ?? Promise.resolve();
      let releaseQueue: () => void = () => undefined;
      const current = new Promise<void>((resolve) => {
        releaseQueue = resolve;
      });
      const queued = previous.catch(() => undefined).then(() => current);
      localUsbRequestQueues.current[deviceId] = queued;
      await previous.catch(() => undefined);
      try {
        const request = { id: nextJsonlRequestId(), method, params };
        const response =
          target.kind === "devd_device"
            ? await sendDevdLocalUsbJsonlRequest(
                agent,
                target.deviceId,
                request,
              )
            : await sendLocalUsbJsonlRequest(agent, target.portPath, request);
        const envelope = response as JsonlEnvelope<T>;
        if (envelope?.ok && envelope.result !== undefined) {
          return { ok: true, value: envelope.result };
        }
        return {
          ok: false,
          error: {
            kind: "api_error",
            status: 500,
            code: envelope?.error?.code ?? "local_usb_error",
            message: envelope?.error?.message ?? "Local USB request failed",
            retryable: envelope?.error?.retryable ?? false,
          },
        };
      } catch (err) {
        if (shouldResetLocalUsbConnectionCache(err)) {
          localUsbAgent.current = null;
          delete localUsbPortByDevice.current[deviceId];
        }
        return {
          ok: false,
          error: localUsbErrorToDeviceApiError(err),
        };
      } finally {
        releaseQueue();
        if (localUsbRequestQueues.current[deviceId] === queued) {
          delete localUsbRequestQueues.current[deviceId];
        }
      }
    },
    [findLocalUsbTarget, getLocalUsbAgent],
  );

  const requestWebSerial = useCallback(
    async <T,>(
      deviceId: string,
      method: string,
      params?: Record<string, unknown>,
    ): Promise<Result<T>> => {
      const transport = getWebSerialDeviceTransport(deviceId);
      if (!transport) {
        return {
          ok: false,
          error: { kind: "offline", message: "Web Serial not connected" },
        };
      }
      try {
        const response = await transport.request({
          id: nextJsonlRequestId(),
          method,
          params,
        });
        const envelope = response as JsonlEnvelope<T>;
        if (envelope?.ok && envelope.result !== undefined) {
          return { ok: true, value: envelope.result };
        }
        return {
          ok: false,
          error: {
            kind: "api_error",
            status: 500,
            code: envelope?.error?.code ?? "web_serial_error",
            message: envelope?.error?.message ?? "Web Serial request failed",
            retryable: envelope?.error?.retryable ?? false,
          },
        };
      } catch (err) {
        forgetWebSerialDeviceTransport(deviceId);
        return {
          ok: false,
          error: {
            kind: "offline",
            message:
              err instanceof Error ? err.message : "Web Serial request failed",
          },
        };
      }
    },
    [],
  );

  const requestTransport = useCallback(
    async <T,>(
      deviceId: string,
      baseUrl: string,
      transport: DeviceTransport,
      method: string,
      params?: Record<string, unknown>,
    ): Promise<Result<T>> => {
      if (transport === "http") {
        if (method === "ports.get") {
          return getPorts(baseUrl) as Promise<Result<T>>;
        }
        if (method === "info") {
          return getDeviceInfo(baseUrl) as Promise<Result<T>>;
        }
        if (method === "wifi.get") {
          return getWifiConfig(baseUrl) as Promise<Result<T>>;
        }
        if (method === "wifi.set") {
          return setWifiConfig(baseUrl, {
            ssid: String(params?.ssid ?? ""),
            psk: String(params?.psk ?? ""),
          }) as Promise<Result<T>>;
        }
        if (method === "wifi.clear") {
          return clearWifiConfig(baseUrl) as Promise<Result<T>>;
        }
        if (method === "reboot") {
          return rebootDevice(baseUrl) as Promise<Result<T>>;
        }
        if (method === "port.power_set") {
          return setPortPower(
            baseUrl,
            params?.port as PortId,
            Boolean(params?.enabled),
          ) as Promise<Result<T>>;
        }
        if (method === "port.replug") {
          return replugPort(baseUrl, params?.port as PortId) as Promise<
            Result<T>
          >;
        }
      }
      if (transport === "web_serial") {
        return requestWebSerial<T>(deviceId, method, params);
      }
      return requestLocalUsb<T>(deviceId, method, params);
    },
    [requestLocalUsb, requestWebSerial],
  );

  const markChannelResult = useCallback(
    (deviceId: string, transport: DeviceTransport, res: Result<unknown>) => {
      setRuntimeById((prev) => {
        const current = prev[deviceId];
        if (!current) {
          return prev;
        }
        return {
          ...prev,
          [deviceId]: {
            ...current,
            channels: {
              ...current.channels,
              [transport]: {
                lastOkAt: res.ok
                  ? Date.now()
                  : current.channels[transport].lastOkAt,
                lastError: res.ok ? null : res.error,
              },
            },
          },
        };
      });
    },
    [],
  );

  const orderedTransports = useCallback(
    (deviceId: string): DeviceTransport[] => {
      const preferred = preferredTransportByDevice.current[deviceId];
      const currentRuntime = runtimeById[deviceId];
      const currentActive = preferred ?? currentRuntime?.transport;
      const active =
        currentActive &&
        (preferred || currentRuntime?.channels[currentActive]?.lastOkAt)
          ? currentActive
          : null;
      const stored = devices.find((device) => device.id === deviceId);
      const devdDeviceId = stored ? localUsbDeviceIdForDevice(stored) : null;
      const httpLinked =
        !!stored?.transports?.httpBaseUrl ||
        (stored ? !localUsbDeviceIdForDevice(stored) : false);
      const localUsbLinked =
        !!localUsbPortByDevice.current[deviceId] ||
        !!getLocalUsbDeviceLink(deviceId) ||
        !!devdDeviceId;
      return devdDeviceId
        ? uniqueTransports([
            active,
            "local_usb",
            httpLinked ? "http" : null,
            "web_serial",
          ])
        : uniqueTransports([
            active,
            httpLinked ? "http" : null,
            "web_serial",
            localUsbLinked ? "local_usb" : null,
          ]);
    },
    [devices, runtimeById],
  );

  const pollDevice = useCallback(
    async (deviceId: string, baseUrl: string) => {
      if (inflight.current.has(deviceId)) {
        return;
      }
      inflight.current.add(deviceId);
      try {
        let res: Result<PortsResponse> | null = null;
        let transport: DeviceTransport | null = null;
        for (const candidate of orderedTransports(deviceId)) {
          const candidateRes = await requestTransport<PortsResponse>(
            deviceId,
            candidate === "http"
              ? httpBaseUrlForDevice(
                  devices.find((device) => device.id === deviceId) ?? {
                    id: deviceId,
                    name: deviceId,
                    baseUrl,
                  },
                )
              : baseUrl,
            candidate,
            "ports.get",
          );
          markChannelResult(deviceId, candidate, candidateRes);
          if (candidateRes.ok) {
            res = candidateRes;
            transport = candidate;
            preferredTransportByDevice.current[deviceId] = candidate;
            break;
          }
          res = candidateRes;
        }
        if (!res) {
          return;
        }
        setRuntimeById((prev) => {
          const current = prev[deviceId];
          if (!current) {
            return prev;
          }
          if (res.ok) {
            const ports = normalizePortsResponse(res.value);
            if (!ports) {
              return {
                ...prev,
                [deviceId]: {
                  ...current,
                  lastError: {
                    kind: "invalid_response",
                    message: "missing port1..port4 in /api/v1/ports response",
                  },
                },
              };
            }
            return {
              ...prev,
              [deviceId]: {
                ...current,
                lastOkAt: Date.now(),
                lastError: null,
                transport,
                hub: normalizeHubSnapshot(res.value.hub ?? null),
                ports,
              },
            };
          }
          delete preferredTransportByDevice.current[deviceId];
          return {
            ...prev,
            [deviceId]: {
              ...current,
              lastError: res.error,
              transport: current.transport,
            },
          };
        });
      } finally {
        inflight.current.delete(deviceId);
      }
    },
    [devices, markChannelResult, orderedTransports, requestTransport],
  );
  const pollDeviceRef = useRef(pollDevice);

  useEffect(() => {
    pollDeviceRef.current = pollDevice;
  }, [pollDevice]);

  useEffect(() => {
    return subscribeLocalUsbDeviceLinks((link) => {
      localUsbPortByDevice.current[link.deviceId] = link.portPath;
      preferredTransportByDevice.current[link.deviceId] = "local_usb";
      const device = devices.find((d) => d.id === link.deviceId);
      if (device) {
        void pollDevice(link.deviceId, httpBaseUrlForDevice(device));
      }
    });
  }, [devices, pollDevice]);

  useEffect(() => {
    return subscribeWebSerialDeviceLinks((link) => {
      preferredTransportByDevice.current[link.deviceId] = "web_serial";
      const device = devices.find((d) => d.id === link.deviceId);
      if (device) {
        void pollDevice(link.deviceId, httpBaseUrlForDevice(device));
      }
    });
  }, [devices, pollDevice]);

  useEffect(() => {
    return subscribeNetworkDeviceLinks((link) => {
      markChannelResult(link.deviceId, "http", {
        ok: true,
        value: { baseUrl: link.baseUrl },
      });
      const currentTransport = runtimeById[link.deviceId]?.transport;
      if (!currentTransport) {
        preferredTransportByDevice.current[link.deviceId] = "http";
      }
      const device = devices.find((d) => d.id === link.deviceId);
      const previousHttp = device?.transports?.httpBaseUrl ?? device?.baseUrl;
      if (device && previousHttp !== link.baseUrl) {
        void persistDevice({
          ...device,
          transports: {
            ...device.transports,
            httpBaseUrl: link.baseUrl,
          },
        });
      }
      void pollDevice(link.deviceId, link.baseUrl);
    });
  }, [devices, markChannelResult, persistDevice, pollDevice, runtimeById]);

  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      const nextNow = Date.now();
      setNow(nextNow);
      if (cancelled) {
        return;
      }
      await Promise.all(
        devices.map((d) =>
          pollDeviceRef.current(d.id, httpBaseUrlForDevice(d)),
        ),
      );
    };

    void tick();
    const id = window.setInterval(tick, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [devices]);

  const setPending = useCallback(
    (deviceId: string, portId: PortId, value: boolean) => {
      setRuntimeById((prev) => {
        const current = prev[deviceId];
        if (!current) {
          return prev;
        }
        return {
          ...prev,
          [deviceId]: {
            ...current,
            pending: { ...current.pending, [portId]: value },
          },
        };
      });
    },
    [],
  );

  const refreshDevice = useCallback(
    async (deviceId: string) => {
      const device = devices.find((d) => d.id === deviceId);
      if (!device) {
        return;
      }
      await pollDevice(deviceId, httpBaseUrlForDevice(device));
    },
    [devices, pollDevice],
  );

  const waitForPowerEcho = useCallback(
    async (
      deviceId: string,
      portId: PortId,
      enabled: boolean,
    ): Promise<boolean> => {
      for (let attempt = 0; attempt < POWER_ECHO_ATTEMPTS; attempt += 1) {
        await refreshDevice(deviceId);
        await delay(POWER_ECHO_DELAY_MS);
        const echoed =
          runtimeByIdRef.current[deviceId]?.ports?.[portId]?.state
            .power_enabled === enabled;
        if (echoed) {
          return true;
        }
      }
      return false;
    },
    [refreshDevice],
  );

  const runDeviceCommand = useCallback(
    async <T,>(
      deviceId: string,
      method: string,
      params?: Record<string, unknown>,
      allowedTransports?: DeviceTransport[],
    ): Promise<Result<T>> => {
      const device = devices.find((d) => d.id === deviceId);
      if (!device) {
        return {
          ok: false,
          error: { kind: "offline", message: "device not found" },
        };
      }
      let res: Result<T> | null = null;
      const transports = allowedTransports
        ? orderedTransports(deviceId).filter((transport) =>
            allowedTransports.includes(transport),
          )
        : orderedTransports(deviceId);
      if (transports.length === 0) {
        return {
          ok: false,
          error: {
            kind: "offline",
            message: "Web Serial or Local USB connection required",
          },
        };
      }
      for (const transport of transports) {
        const candidate = await requestTransport<T>(
          deviceId,
          transport === "http" ? httpBaseUrlForDevice(device) : device.baseUrl,
          transport,
          method,
          params,
        );
        markChannelResult(deviceId, transport, candidate);
        if (candidate.ok) {
          preferredTransportByDevice.current[deviceId] = transport;
          res = candidate;
          break;
        }
        res = candidate;
      }
      if (!res) {
        return {
          ok: false,
          error: { kind: "offline", message: "device has no active transport" },
        };
      }
      return res;
    },
    [devices, markChannelResult, orderedTransports, requestTransport],
  );

  const deviceInfo = useCallback(
    async (deviceId: string): Promise<Result<DeviceInfoResponse>> => {
      const res = await runDeviceCommand<DeviceInfoResponse>(deviceId, "info");
      if (res.ok && !isDeviceInfoResponse(res.value)) {
        return {
          ok: false,
          error: {
            kind: "invalid_response",
            message: "info response is missing device identity",
          },
        };
      }
      if (res.ok && res.value.device.wifi) {
        if (announceWifiHttpLink(deviceId, res.value.device.wifi)) {
          await refreshDevices();
        }
      }
      return res;
    },
    [refreshDevices, runDeviceCommand],
  );

  const wifiConfig = useCallback(
    async (deviceId: string): Promise<Result<WifiConfigResponse>> => {
      const res = await runDeviceCommand<WifiConfigResponse>(
        deviceId,
        "wifi.get",
      );
      if (res.ok) {
        if (announceWifiHttpLink(deviceId, res.value)) {
          await refreshDevices();
        }
      }
      return res;
    },
    [refreshDevices, runDeviceCommand],
  );

  const saveWifiConfig = useCallback(
    async (
      deviceId: string,
      input: WifiConfigInput,
    ): Promise<Result<WifiMutationResponse>> => {
      const res = await runDeviceCommand<WifiMutationResponse>(
        deviceId,
        "wifi.set",
        input,
        ["web_serial", "local_usb"],
      );
      if (res.ok) {
        const linkedWifiHttp = announceWifiHttpLink(deviceId, res.value);
        await refreshDevice(deviceId);
        if (linkedWifiHttp) {
          await refreshDevices();
        }
      }
      return res;
    },
    [refreshDevice, refreshDevices, runDeviceCommand],
  );

  const clearWifi = useCallback(
    async (deviceId: string): Promise<Result<WifiMutationResponse>> => {
      const res = await runDeviceCommand<WifiMutationResponse>(
        deviceId,
        "wifi.clear",
        undefined,
        ["web_serial", "local_usb"],
      );
      if (res.ok) {
        await refreshDevice(deviceId);
        await refreshDevices();
      }
      return res;
    },
    [refreshDevice, refreshDevices, runDeviceCommand],
  );

  const reboot = useCallback(
    async (deviceId: string): Promise<Result<RebootResponse>> => {
      return runDeviceCommand<RebootResponse>(deviceId, "reboot", undefined, [
        "web_serial",
        "local_usb",
      ]);
    },
    [runDeviceCommand],
  );

  const handleApiErrorToast = useCallback(
    (deviceName: string, label: string, err: DeviceApiError) => {
      if (err.kind === "busy") {
        pushToast({
          message: `${deviceName}: ${label} is busy`,
          variant: "warning",
        });
        return;
      }
      pushToast({
        message: `${deviceName}: ${label} error (${err.kind})`,
        variant: "error",
      });
    },
    [pushToast],
  );

  const setPower = useCallback(
    async (deviceId: string, portId: PortId, enabled: boolean) => {
      const device = devices.find((d) => d.id === deviceId);
      if (!device) {
        return;
      }

      const label = portLabel(portId);
      const transports = orderedTransports(deviceId).filter(
        (transport) => transport === "web_serial" || transport === "local_usb",
      );
      if (transports.length === 0) {
        pushToast({
          message: `${device.name}: ${label} requires Web Serial or Local USB`,
          variant: "warning",
        });
        return;
      }
      setPending(deviceId, portId, true);
      try {
        let res: Result<{ accepted: true }> | null = null;
        for (const transport of transports) {
          const candidate = await requestTransport<{ accepted: true }>(
            deviceId,
            device.baseUrl,
            transport,
            "port.power_set",
            {
              port: portId,
              enabled,
            },
          );
          markChannelResult(deviceId, transport, candidate);
          if (candidate.ok) {
            preferredTransportByDevice.current[deviceId] = transport;
            res = candidate;
            break;
          }
          res = candidate;
        }
        if (!res) {
          return;
        }
        if (res.ok) {
          const echoed = await waitForPowerEcho(deviceId, portId, enabled);
          pushToast({
            message: echoed
              ? `${device.name}: ${label} power set`
              : `${device.name}: ${label} power accepted, refresh lagging`,
            variant: echoed ? "success" : "warning",
          });
          return;
        }
        handleApiErrorToast(device.name, label, res.error);
      } finally {
        setPending(deviceId, portId, false);
      }
    },
    [
      devices,
      handleApiErrorToast,
      pushToast,
      markChannelResult,
      orderedTransports,
      requestTransport,
      setPending,
      waitForPowerEcho,
    ],
  );

  const replug = useCallback(
    async (deviceId: string, portId: PortId) => {
      const device = devices.find((d) => d.id === deviceId);
      if (!device) {
        return;
      }

      const label = portLabel(portId);
      const transports = orderedTransports(deviceId).filter(
        (transport) => transport === "web_serial" || transport === "local_usb",
      );
      if (transports.length === 0) {
        pushToast({
          message: `${device.name}: ${label} requires Web Serial or Local USB`,
          variant: "warning",
        });
        return;
      }
      setPending(deviceId, portId, true);
      try {
        let res: Result<{ accepted: true }> | null = null;
        for (const transport of transports) {
          const candidate = await requestTransport<{ accepted: true }>(
            deviceId,
            device.baseUrl,
            transport,
            "port.replug",
            {
              port: portId,
            },
          );
          markChannelResult(deviceId, transport, candidate);
          if (candidate.ok) {
            preferredTransportByDevice.current[deviceId] = transport;
            res = candidate;
            break;
          }
          res = candidate;
        }
        if (!res) {
          return;
        }
        if (res.ok) {
          pushToast({
            message: `${device.name}: ${label} replug accepted`,
            variant: "success",
          });
          await refreshDevice(deviceId);
          return;
        }
        handleApiErrorToast(device.name, label, res.error);
      } finally {
        setPending(deviceId, portId, false);
      }
    },
    [
      devices,
      handleApiErrorToast,
      pushToast,
      refreshDevice,
      markChannelResult,
      orderedTransports,
      requestTransport,
      setPending,
    ],
  );

  const value = useMemo<DeviceRuntimeContextValue>(() => {
    const connectionState = (deviceId: string): ConnectionState => {
      const rt = runtimeById[deviceId];
      if (!rt || rt.lastOkAt === null) {
        return "unknown";
      }
      return now - rt.lastOkAt >= OFFLINE_THRESHOLD_MS ? "offline" : "online";
    };

    const lastOkAt = (deviceId: string): number | null =>
      runtimeById[deviceId]?.lastOkAt ?? null;

    const lastErrorLabel = (deviceId: string): string | null => {
      const rt = runtimeById[deviceId];
      if (!rt?.lastError) {
        return null;
      }
      return shortApiError(rt.lastError);
    };

    const transport = (deviceId: string): DeviceTransport | null =>
      runtimeById[deviceId]?.transport ?? null;

    const usbWriteTransport = (deviceId: string): DeviceTransport | null => {
      const active = runtimeById[deviceId]?.transport ?? null;
      const stored = devices.find((device) => device.id === deviceId);
      if (active === "web_serial" || active === "local_usb") {
        return active;
      }
      if (getWebSerialDeviceTransport(deviceId)) {
        return "web_serial";
      }
      if (
        localUsbPortByDevice.current[deviceId] ||
        getLocalUsbDeviceLink(deviceId) ||
        (stored ? localUsbDeviceIdForDevice(stored) : null)
      ) {
        return "local_usb";
      }
      return null;
    };

    const wifiManagementTransport = usbWriteTransport;

    const channelState = (
      deviceId: string,
      transport: DeviceTransport,
    ): ConnectionState => {
      const channel = runtimeById[deviceId]?.channels[transport];
      if (!channel?.lastOkAt) {
        return "unknown";
      }
      return now - channel.lastOkAt >= OFFLINE_THRESHOLD_MS
        ? "offline"
        : "online";
    };

    const hub = (deviceId: string): HubState | null =>
      runtimeById[deviceId]?.hub ?? null;

    const port = (deviceId: string, portId: PortId): Port | null =>
      runtimeById[deviceId]?.ports?.[portId] ?? null;

    const pending = (deviceId: string, portId: PortId): boolean =>
      runtimeById[deviceId]?.pending?.[portId] ?? false;

    return {
      now,
      runtimeById,
      connectionState,
      lastOkAt,
      lastErrorLabel,
      transport,
      usbWriteTransport,
      wifiManagementTransport,
      channelState,
      hub,
      port,
      pending,
      refreshDevice,
      deviceInfo,
      wifiConfig,
      saveWifiConfig,
      clearWifiConfig: clearWifi,
      rebootDevice: reboot,
      setPower,
      replug,
    };
  }, [
    clearWifi,
    deviceInfo,
    devices,
    now,
    reboot,
    refreshDevice,
    replug,
    runtimeById,
    saveWifiConfig,
    setPower,
    wifiConfig,
  ]);

  return (
    <DeviceRuntimeContext.Provider value={value}>
      {children}
    </DeviceRuntimeContext.Provider>
  );
}

export function useDeviceRuntime(): DeviceRuntimeContextValue {
  const ctx = useContext(DeviceRuntimeContext);
  if (!ctx) {
    throw new Error(
      "useDeviceRuntime must be used within <DeviceRuntimeProvider>",
    );
  }
  return ctx;
}
