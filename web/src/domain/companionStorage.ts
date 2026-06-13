import type { ThemeId } from "../app/theme";
import { agentFetch, type CompanionBridge } from "./companionBridge";
import {
  type AddDeviceInput,
  normalizeBaseUrl,
  type StoredDevice,
} from "./devices";

const VALID_THEMES = ["isohub", "isohub-dark", "system"] as const;

type StorageError = { code?: string; message: string };

type StorageResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: StorageError };

type StorageDevicesResponse = {
  devices: StoredDevice[];
};

type StorageDeviceResponse = {
  device: StoredDevice;
};

type StorageSettingsResponse = {
  settings: { theme: ThemeId };
};

type StorageMigrateResponse = {
  migrated: boolean;
  imported?: { devices: number; settings: boolean };
  reason?: string;
};

type CompanionStorageExport = {
  schema_version: number;
  devices: StoredDevice[];
  settings: { theme?: ThemeId };
  meta?: {
    migrated_from_localstorage_at?: string;
    last_corrupt_at?: string;
    last_corrupt_reason?: string;
  };
};

function isThemeId(value: unknown): value is ThemeId {
  return typeof value === "string" && VALID_THEMES.includes(value as ThemeId);
}

function parseStoredDevice(value: unknown): StoredDevice | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.id !== "string") {
    return null;
  }
  if (typeof record.name !== "string") {
    return null;
  }
  if (typeof record.baseUrl !== "string") {
    return null;
  }
  const normalized = normalizeBaseUrl(record.baseUrl);
  const transports =
    record.transports && typeof record.transports === "object"
      ? (record.transports as Record<string, unknown>)
      : null;
  const httpBaseUrl =
    typeof transports?.httpBaseUrl === "string"
      ? normalizeBaseUrl(transports.httpBaseUrl)
      : null;
  return {
    id: record.id,
    name: record.name,
    baseUrl: normalized.ok ? normalized.baseUrl : record.baseUrl,
    transports: transports
      ? {
          httpBaseUrl: httpBaseUrl?.ok
            ? httpBaseUrl.baseUrl
            : typeof transports.httpBaseUrl === "string"
              ? transports.httpBaseUrl
              : undefined,
          localUsbDeviceId:
            typeof transports.localUsbDeviceId === "string"
              ? transports.localUsbDeviceId
              : undefined,
          webSerialLabel:
            typeof transports.webSerialLabel === "string"
              ? transports.webSerialLabel
              : undefined,
        }
      : undefined,
    lastSeenAt:
      typeof record.lastSeenAt === "string" ? record.lastSeenAt : undefined,
  };
}

async function readStorageError(res: Response): Promise<StorageError> {
  try {
    const json = (await res.json()) as unknown;
    if (json && typeof json === "object") {
      const obj = json as Record<string, unknown>;
      const error = obj.error as Record<string, unknown> | undefined;
      const message =
        error && typeof error.message === "string"
          ? error.message
          : `HTTP ${res.status}`;
      const code =
        error && typeof error.code === "string" ? error.code : undefined;
      return { code, message };
    }
  } catch {
    // ignore
  }
  return { message: `HTTP ${res.status}` };
}

export async function fetchStoredDevices(
  agent: CompanionBridge,
): Promise<StorageResult<StoredDevice[]>> {
  const res = await agentFetch(agent, "/api/v1/storage/devices");
  if (!res.ok) {
    return { ok: false, error: await readStorageError(res) };
  }
  const json = (await res.json()) as unknown;
  const obj = json as StorageDevicesResponse | undefined;
  const devicesRaw = Array.isArray(obj?.devices) ? obj.devices : [];
  const devices = devicesRaw
    .map((d) => parseStoredDevice(d))
    .filter((d): d is StoredDevice => Boolean(d));
  return { ok: true, value: devices };
}

export async function upsertStoredDevice(
  agent: CompanionBridge,
  input: AddDeviceInput,
): Promise<StorageResult<StoredDevice>> {
  const res = await agentFetch(agent, "/api/v1/storage/devices", {
    method: "POST",
    body: JSON.stringify({ device: input }),
  });
  if (!res.ok) {
    return { ok: false, error: await readStorageError(res) };
  }
  const json = (await res.json()) as unknown;
  const obj = json as StorageDeviceResponse | undefined;
  const device = obj?.device ? parseStoredDevice(obj.device) : null;
  if (!device) {
    return { ok: false, error: { message: "invalid response" } };
  }
  return { ok: true, value: device };
}

export async function deleteStoredDevice(
  agent: CompanionBridge,
  deviceId: string,
): Promise<StorageResult<boolean>> {
  const res = await agentFetch(agent, `/api/v1/storage/devices/${deviceId}`, {
    method: "DELETE",
  });
  if (!res.ok) {
    return { ok: false, error: await readStorageError(res) };
  }
  return { ok: true, value: true };
}

export async function fetchStoredTheme(
  agent: CompanionBridge,
): Promise<StorageResult<ThemeId>> {
  const res = await agentFetch(agent, "/api/v1/storage/settings");
  if (!res.ok) {
    return { ok: false, error: await readStorageError(res) };
  }
  const json = (await res.json()) as unknown;
  const obj = json as StorageSettingsResponse | undefined;
  const theme = obj?.settings?.theme;
  if (!isThemeId(theme)) {
    return { ok: true, value: "isohub" };
  }
  return { ok: true, value: theme };
}

export async function updateStoredTheme(
  agent: CompanionBridge,
  theme: ThemeId,
): Promise<StorageResult<ThemeId>> {
  const res = await agentFetch(agent, "/api/v1/storage/settings", {
    method: "PUT",
    body: JSON.stringify({ settings: { theme } }),
  });
  if (!res.ok) {
    return { ok: false, error: await readStorageError(res) };
  }
  const json = (await res.json()) as unknown;
  const obj = json as StorageSettingsResponse | undefined;
  const nextTheme = obj?.settings?.theme;
  if (!isThemeId(nextTheme)) {
    return { ok: false, error: { message: "invalid response" } };
  }
  return { ok: true, value: nextTheme };
}

export async function migrateFromLocalStorage(
  agent: CompanionBridge,
  payload: {
    devices?: StoredDevice[];
    settings?: { theme?: ThemeId };
  },
): Promise<StorageResult<StorageMigrateResponse>> {
  const res = await agentFetch(agent, "/api/v1/storage/migrate/localstorage", {
    method: "POST",
    body: JSON.stringify({
      source: "localStorage",
      devices: payload.devices,
      settings: payload.settings,
    }),
  });
  if (!res.ok) {
    return { ok: false, error: await readStorageError(res) };
  }
  const json = (await res.json()) as unknown;
  const obj = json as StorageMigrateResponse | undefined;
  if (typeof obj?.migrated !== "boolean") {
    return { ok: false, error: { message: "invalid response" } };
  }
  return { ok: true, value: obj };
}

export async function exportStorage(
  agent: CompanionBridge,
): Promise<StorageResult<CompanionStorageExport>> {
  const res = await agentFetch(agent, "/api/v1/storage/export");
  if (!res.ok) {
    return { ok: false, error: await readStorageError(res) };
  }
  const json = (await res.json()) as unknown;
  const obj = json as CompanionStorageExport | undefined;
  if (!obj || typeof obj.schema_version !== "number") {
    return { ok: false, error: { message: "invalid response" } };
  }
  return { ok: true, value: obj };
}

export async function resetStorage(
  agent: CompanionBridge,
): Promise<StorageResult<boolean>> {
  const res = await agentFetch(agent, "/api/v1/storage/reset", {
    method: "POST",
    body: JSON.stringify({}),
  });
  if (!res.ok) {
    return { ok: false, error: await readStorageError(res) };
  }
  return { ok: true, value: true };
}
