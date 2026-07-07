export const DEVICES_STORAGE_KEY = "isolarail.devices";

export type StoredDevice = {
  id: string;
  name: string;
  baseUrl: string;
  lastSeenAt?: string;
  transports?: {
    httpBaseUrl?: string;
    localUsbDeviceId?: string;
    webSerialLabel?: string;
  };
};

export type AddDeviceInput = {
  name: string;
  baseUrl: string;
  id?: string;
  transports?: StoredDevice["transports"];
};

export type AddDeviceValidationErrors = {
  name?: string;
  baseUrl?: string;
  id?: string;
};

export type AddDeviceValidationResult =
  | { ok: true; device: StoredDevice }
  | { ok: false; errors: AddDeviceValidationErrors };

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isStoredDevice(value: unknown): value is StoredDevice {
  if (!value || typeof value !== "object") {
    return false;
  }

  const record = value as Record<string, unknown>;
  return (
    isNonEmptyString(record.id) &&
    isNonEmptyString(record.name) &&
    isNonEmptyString(record.baseUrl) &&
    (record.lastSeenAt === undefined || typeof record.lastSeenAt === "string")
  );
}

function parseStoredDeviceTransports(
  value: unknown,
): StoredDevice["transports"] {
  if (!value || typeof value !== "object") {
    return undefined;
  }
  const record = value as Record<string, unknown>;
  const transports: NonNullable<StoredDevice["transports"]> = {};
  if (typeof record.httpBaseUrl === "string") {
    const normalized = normalizeBaseUrl(record.httpBaseUrl);
    transports.httpBaseUrl = normalized.ok
      ? normalized.baseUrl
      : record.httpBaseUrl;
  }
  if (typeof record.localUsbDeviceId === "string") {
    transports.localUsbDeviceId = record.localUsbDeviceId;
  }
  if (typeof record.webSerialLabel === "string") {
    transports.webSerialLabel = record.webSerialLabel;
  }
  return Object.keys(transports).length > 0 ? transports : undefined;
}

export function normalizeBaseUrl(
  raw: string,
): { ok: true; baseUrl: string } | { ok: false; error: string } {
  const trimmed = raw.trim();
  if (trimmed.length === 0) {
    return { ok: false, error: "Base URL is required" };
  }

  let url: URL;
  try {
    url = new URL(trimmed);
  } catch {
    return { ok: false, error: "Base URL must be a valid URL" };
  }

  if (url.protocol !== "http:" && url.protocol !== "https:") {
    return { ok: false, error: "Base URL must start with http:// or https://" };
  }

  return { ok: true, baseUrl: url.origin };
}

export function validateAddDeviceInput(
  input: AddDeviceInput,
  existingDeviceIds: Iterable<string> = [],
  existingBaseUrls: Iterable<string> = [],
): AddDeviceValidationResult {
  const errors: AddDeviceValidationErrors = {};

  const name = input.name.trim();
  if (name.length === 0) {
    errors.name = "Name is required";
  }

  const baseUrlResult = normalizeBaseUrl(input.baseUrl);
  if (!baseUrlResult.ok) {
    errors.baseUrl = baseUrlResult.error;
  }

  if (baseUrlResult.ok) {
    const existing = new Set(
      Array.from(existingBaseUrls, (v) => v.trim()).filter(Boolean),
    );
    if (existing.has(baseUrlResult.baseUrl)) {
      errors.baseUrl = "Base URL already exists";
    }
  }

  const idRaw = input.id;
  const id = idRaw === undefined ? undefined : idRaw.trim();
  if (id !== undefined && id.length === 0) {
    errors.id = "ID cannot be blank";
  }

  if (id) {
    const existing = new Set(existingDeviceIds);
    if (existing.has(id)) {
      errors.id = "ID already exists";
    }
  }

  if (errors.name || errors.baseUrl || errors.id) {
    return { ok: false, errors };
  }

  const finalId = id ?? crypto.randomUUID().split("-")[0];

  return {
    ok: true,
    device: {
      id: finalId,
      name,
      baseUrl: baseUrlResult.ok ? baseUrlResult.baseUrl : input.baseUrl,
      transports: parseStoredDeviceTransports(input.transports),
    },
  };
}

export function loadStoredDevices(): StoredDevice[] {
  if (typeof window === "undefined") {
    return [];
  }

  const raw = window.localStorage.getItem(DEVICES_STORAGE_KEY);
  if (!raw) {
    return [];
  }

  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }

    return parsed.filter(isStoredDevice).map((d) => {
      const normalized = normalizeBaseUrl(d.baseUrl);
      return {
        ...d,
        baseUrl: normalized.ok ? normalized.baseUrl : d.baseUrl,
        transports: parseStoredDeviceTransports(d.transports),
      };
    });
  } catch {
    return [];
  }
}

export function saveStoredDevices(devices: StoredDevice[]): void {
  if (typeof window === "undefined") {
    return;
  }

  window.localStorage.setItem(DEVICES_STORAGE_KEY, JSON.stringify(devices));
}
