import {
  DEVICES_STORAGE_KEY,
  normalizeBaseUrl,
  type StoredDevice,
} from "../domain/devices";
import { THEME_STORAGE_KEY, type ThemeId } from "./theme";

const VALID_THEMES = ["isohub", "isohub-dark", "system"] as const;

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
  return {
    id: record.id,
    name: record.name,
    baseUrl: normalized.ok ? normalized.baseUrl : record.baseUrl,
    lastSeenAt:
      typeof record.lastSeenAt === "string" ? record.lastSeenAt : undefined,
  };
}

function readLocalStorageDevices(): StoredDevice[] | null {
  if (typeof window === "undefined") {
    return null;
  }
  const raw = window.localStorage.getItem(DEVICES_STORAGE_KEY);
  if (!raw) {
    return null;
  }
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return null;
    }
    const devices = parsed
      .map((item) => parseStoredDevice(item))
      .filter((item): item is StoredDevice => Boolean(item));
    return devices;
  } catch {
    return null;
  }
}

function readLocalStorageTheme(): ThemeId | null {
  if (typeof window === "undefined") {
    return null;
  }
  const raw = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (!raw) {
    return null;
  }
  try {
    const parsed: unknown = JSON.parse(raw);
    return isThemeId(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function readMigrationPayload(): {
  devices?: StoredDevice[];
  settings?: { theme?: ThemeId };
} | null {
  const devices = readLocalStorageDevices();
  const theme = readLocalStorageTheme();
  if (!devices && !theme) {
    return null;
  }
  const payload: { devices?: StoredDevice[]; settings?: { theme?: ThemeId } } =
    {};
  if (devices && devices.length > 0) {
    payload.devices = devices;
  }
  if (theme) {
    payload.settings = { theme };
  }
  return Object.keys(payload).length > 0 ? payload : null;
}
