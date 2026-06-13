import type {
  DeviceApiError,
  DeviceInfoResponse,
  RebootResponse,
  Result,
  WifiConfigInput,
  WifiConfigResponse,
  WifiMutationResponse,
} from "../domain/deviceApi";
import type { StoredDevice } from "../domain/devices";
import {
  devdLocalUsbDeviceIdFromBaseUrl,
  LocalUsbAgentHttpError,
} from "../domain/hardwareConsole";
import type { CanonicalPortId, HubState, Port, PortId } from "../domain/ports";
import { CANONICAL_PORT_IDS } from "../domain/ports";

export type ConnectionState = "online" | "offline" | "unknown";
export type DeviceTransport = "http" | "web_serial" | "local_usb";

export type ChannelRuntime = {
  lastOkAt: number | null;
  lastError: DeviceApiError | null;
};

export type DeviceRuntime = {
  lastOkAt: number | null;
  lastError: DeviceApiError | null;
  transport: DeviceTransport | null;
  channels: Record<DeviceTransport, ChannelRuntime>;
  hub: HubState | null;
  ports: Record<CanonicalPortId, Port> | null;
  pending: Record<CanonicalPortId, boolean>;
};

export type DeviceRuntimeContextValue = {
  now: number;
  runtimeById: Record<string, DeviceRuntime>;
  connectionState: (deviceId: string) => ConnectionState;
  lastOkAt: (deviceId: string) => number | null;
  lastErrorLabel: (deviceId: string) => string | null;
  transport: (deviceId: string) => DeviceTransport | null;
  wifiManagementTransport: (deviceId: string) => DeviceTransport | null;
  channelState: (
    deviceId: string,
    transport: DeviceTransport,
  ) => ConnectionState;
  hub: (deviceId: string) => HubState | null;
  port: (deviceId: string, portId: PortId) => Port | null;
  pending: (deviceId: string, portId: PortId) => boolean;
  refreshDevice: (deviceId: string) => Promise<void>;
  deviceInfo: (deviceId: string) => Promise<Result<DeviceInfoResponse>>;
  wifiConfig: (deviceId: string) => Promise<Result<WifiConfigResponse>>;
  saveWifiConfig: (
    deviceId: string,
    input: WifiConfigInput,
  ) => Promise<Result<WifiMutationResponse>>;
  clearWifiConfig: (deviceId: string) => Promise<Result<WifiMutationResponse>>;
  rebootDevice: (deviceId: string) => Promise<Result<RebootResponse>>;
  setPower: (
    deviceId: string,
    portId: PortId,
    enabled: boolean,
  ) => Promise<void>;
  replug: (deviceId: string, portId: PortId) => Promise<void>;
};

const TRANSPORTS: DeviceTransport[] = ["http", "web_serial", "local_usb"];

export function emptyPendingPorts(): Record<CanonicalPortId, boolean> {
  return Object.fromEntries(
    CANONICAL_PORT_IDS.map((portId) => [portId, false]),
  ) as Record<CanonicalPortId, boolean>;
}

export function httpBaseUrlForDevice(device: StoredDevice): string {
  return device.transports?.httpBaseUrl ?? device.baseUrl;
}

export function localUsbDeviceIdForDevice(device: StoredDevice): string | null {
  return (
    device.transports?.localUsbDeviceId ??
    devdLocalUsbDeviceIdFromBaseUrl(device.baseUrl)
  );
}

export function shortApiError(err: DeviceApiError): string {
  if (err.kind === "offline") {
    return "Offline: device unreachable";
  }
  if (err.kind === "preflight_blocked") {
    return "Blocked: CORS/PNA preflight";
  }
  if (err.kind === "invalid_response") {
    return "Invalid response";
  }
  if (err.kind === "busy") {
    return "Busy";
  }
  return `API error: ${err.code}`;
}

export function createEmptyChannels(): Record<DeviceTransport, ChannelRuntime> {
  return {
    http: { lastOkAt: null, lastError: null },
    web_serial: { lastOkAt: null, lastError: null },
    local_usb: { lastOkAt: null, lastError: null },
  };
}

export function shouldResetLocalUsbConnectionCache(err: unknown): boolean {
  if (err instanceof LocalUsbAgentHttpError) {
    return false;
  }
  const message = err instanceof Error ? err.message : String(err);
  return !message.includes("serial port is busy");
}

export function localUsbErrorToDeviceApiError(err: unknown): DeviceApiError {
  if (err instanceof LocalUsbAgentHttpError) {
    if (err.status === 409 && err.code === "busy") {
      return { kind: "busy", message: err.message, retryable: true };
    }
    return {
      kind: "api_error",
      status: err.status,
      code: err.code,
      message: err.message,
      retryable: err.retryable,
    };
  }
  return {
    kind: "offline",
    message: err instanceof Error ? err.message : "Local USB request failed",
  };
}

export function uniqueTransports(
  candidates: Array<DeviceTransport | null | undefined>,
): DeviceTransport[] {
  const seen = new Set<DeviceTransport>();
  const ordered: DeviceTransport[] = [];
  for (const candidate of candidates) {
    if (!candidate || seen.has(candidate)) {
      continue;
    }
    if (!TRANSPORTS.includes(candidate)) {
      continue;
    }
    seen.add(candidate);
    ordered.push(candidate);
  }
  return ordered;
}

export function isDeviceInfoResponse(
  value: unknown,
): value is DeviceInfoResponse {
  if (!value || typeof value !== "object") {
    return false;
  }
  const device = (value as { device?: unknown }).device;
  return !!device && typeof device === "object";
}
