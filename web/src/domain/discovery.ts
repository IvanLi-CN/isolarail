import type { AddDeviceValidationErrors } from "./devices";

export type DiscoveredDevice = {
  baseUrl: string;
  device_id?: string;
  hostname?: string;
  fqdn?: string;
  ipv4?: string;
  variant?: string;
  firmware?: { name: string; version: string };
  last_seen_at?: string;
};

export type DiscoveryStatus = "idle" | "scanning" | "ready" | "unavailable";
export type DiscoveryMode = "service" | "scan";

export type LanCandidate = {
  cidr: string;
  label?: string;
  interface?: string;
  ipv4?: string;
  primary?: boolean;
};

export type DiscoverySnapshot = {
  mode: DiscoveryMode;
  status: DiscoveryStatus;
  devices: DiscoveredDevice[];
  error?: string;
  scan?: { cidr: string; done: number; total: number };
  ipScan?: {
    expanded: boolean;
    expandedBy?: "user" | "auto";
    autoExpandAfterMs?: number;
    defaultCidr?: string;
    candidates?: LanCandidate[];
  };
};

export type DiscoveryAction =
  | { type: "reset"; status: DiscoveryStatus; error?: string }
  | {
      type: "set_snapshot";
      snapshot: DiscoverySnapshot;
    }
  | { type: "set_devices"; devices: DiscoveredDevice[] }
  | { type: "set_error"; error: string }
  | { type: "toggle_ip_scan"; expanded: boolean; expandedBy: "user" | "auto" }
  | { type: "start_scan"; cidr: string; total: number }
  | { type: "scan_progress"; done: number }
  | { type: "scan_device"; device: DiscoveredDevice }
  | { type: "scan_done" }
  | { type: "scan_cancelled" };

export function createInitialDiscoverySnapshot({
  status,
  autoExpandAfterMs,
}: {
  status: DiscoveryStatus;
  autoExpandAfterMs?: number;
}): DiscoverySnapshot {
  return {
    mode: "service",
    status,
    devices: [],
    ipScan: {
      expanded: false,
      autoExpandAfterMs,
    },
  };
}

function deviceDedupKey(device: DiscoveredDevice): string {
  if (device.device_id && device.device_id.trim().length > 0) {
    return `id:${device.device_id.trim()}`;
  }
  return `url:${device.baseUrl.trim()}`;
}

export function mergeDiscoveredDevice(
  devices: DiscoveredDevice[],
  device: DiscoveredDevice,
): DiscoveredDevice[] {
  const key = deviceDedupKey(device);
  const out: DiscoveredDevice[] = [];
  let merged = false;
  for (const existing of devices) {
    if (deviceDedupKey(existing) === key) {
      out.push({ ...existing, ...device });
      merged = true;
    } else {
      out.push(existing);
    }
  }
  if (!merged) {
    out.push(device);
  }
  return out;
}

export function reduceDiscoverySnapshot(
  snapshot: DiscoverySnapshot,
  action: DiscoveryAction,
): DiscoverySnapshot {
  switch (action.type) {
    case "reset": {
      return {
        ...snapshot,
        mode: "service",
        status: action.status,
        devices: [],
        error: action.error,
        scan: undefined,
      };
    }
    case "set_snapshot": {
      const prevIpScan = snapshot.ipScan ?? { expanded: false };
      const nextIpScan = action.snapshot.ipScan;
      return {
        ...action.snapshot,
        ipScan: {
          expanded: prevIpScan.expanded,
          expandedBy: prevIpScan.expandedBy,
          autoExpandAfterMs: prevIpScan.autoExpandAfterMs,
          defaultCidr: nextIpScan?.defaultCidr,
          candidates: nextIpScan?.candidates,
        },
      };
    }
    case "set_devices": {
      return { ...snapshot, devices: action.devices };
    }
    case "set_error": {
      return { ...snapshot, error: action.error };
    }
    case "toggle_ip_scan": {
      return {
        ...snapshot,
        ipScan: {
          ...(snapshot.ipScan ?? { expanded: false }),
          expanded: action.expanded,
          expandedBy: action.expandedBy,
        },
      };
    }
    case "start_scan": {
      return {
        ...snapshot,
        mode: "scan",
        status: "scanning",
        error: undefined,
        scan: { cidr: action.cidr, done: 0, total: action.total },
      };
    }
    case "scan_progress": {
      if (!snapshot.scan) {
        return snapshot;
      }
      return {
        ...snapshot,
        scan: { ...snapshot.scan, done: action.done },
      };
    }
    case "scan_device": {
      return {
        ...snapshot,
        devices: mergeDiscoveredDevice(snapshot.devices, action.device),
      };
    }
    case "scan_done": {
      return { ...snapshot, status: "ready" };
    }
    case "scan_cancelled": {
      return {
        ...snapshot,
        status: snapshot.mode === "scan" ? "idle" : snapshot.status,
        scan: undefined,
      };
    }
  }
}

export function isDiscoveredDeviceAdded(
  device: DiscoveredDevice,
  existingDeviceIds: Iterable<string>,
  existingBaseUrls: Iterable<string>,
): boolean {
  const ids = new Set(
    Array.from(existingDeviceIds, (v) => v.trim()).filter(Boolean),
  );
  const baseUrls = new Set(
    Array.from(existingBaseUrls, (v) => v.trim()).filter(Boolean),
  );

  const byId = device.device_id ? ids.has(device.device_id) : false;
  const byUrl = baseUrls.has(device.baseUrl);
  return byId || byUrl;
}

export function applyDiscoveredDeviceToManualForm(
  current: { name: string; baseUrl: string; id: string },
  device: DiscoveredDevice,
): { name: string; baseUrl: string; id: string } {
  const suggestedName =
    current.name.trim().length === 0 && device.hostname
      ? device.hostname
      : current.name;
  const suggestedId =
    current.id.trim().length === 0 && device.device_id
      ? device.device_id
      : current.id;
  return {
    name: suggestedName,
    baseUrl: device.baseUrl,
    id: suggestedId,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object";
}

function nonEmptyString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim().length > 0
    ? value
    : undefined;
}

export function parseDiscoveredDeviceFromApiInfo(
  baseUrlByIp: string,
  value: unknown,
  scannedIpv4: string,
  nowIso: string,
): DiscoveredDevice | null {
  if (!isRecord(value)) {
    return null;
  }
  const device = value.device;
  if (!isRecord(device)) {
    return null;
  }

  const deviceId = nonEmptyString(device.device_id);
  const hostname = nonEmptyString(device.hostname);
  const fqdn = nonEmptyString(device.fqdn);
  const variant = nonEmptyString(device.variant);

  const firmwareRaw = device.firmware;
  const firmware = isRecord(firmwareRaw)
    ? {
        name: nonEmptyString(firmwareRaw.name),
        version: nonEmptyString(firmwareRaw.version),
      }
    : { name: undefined, version: undefined };

  if (!firmware.name || firmware.name !== "iso-usb-hub") {
    return null;
  }

  const wifiRaw = device.wifi;
  const wifiIpv4 =
    isRecord(wifiRaw) &&
    (wifiRaw.ipv4 === null || typeof wifiRaw.ipv4 === "string")
      ? (wifiRaw.ipv4 ?? undefined)
      : undefined;

  const preferredBaseUrl = fqdn?.endsWith(".local")
    ? `http://${fqdn}`
    : baseUrlByIp;

  return {
    baseUrl: preferredBaseUrl,
    device_id: deviceId,
    hostname,
    fqdn,
    ipv4: wifiIpv4 ?? scannedIpv4,
    variant,
    firmware: { name: firmware.name, version: firmware.version ?? "unknown" },
    last_seen_at: nowIso,
  };
}

export function validateCidrInput(raw: string):
  | {
      ok: true;
      cidr: string;
      hosts: string[];
    }
  | { ok: false; error: string; errors?: AddDeviceValidationErrors } {
  const cidr = raw.trim();
  if (cidr.length === 0) {
    return { ok: false, error: "CIDR is required" };
  }

  const parsed = parseCidr(cidr);
  return parsed;
}

export function parseCidr(
  cidr: string,
  maxHosts = 1024,
): { ok: true; cidr: string; hosts: string[] } | { ok: false; error: string } {
  const [ipRaw, prefixRaw] = cidr.split("/");
  if (!ipRaw || !prefixRaw) {
    return { ok: false, error: "CIDR must look like 192.168.1.0/24" };
  }
  const prefix = Number(prefixRaw);
  if (!Number.isInteger(prefix) || prefix < 0 || prefix > 32) {
    return { ok: false, error: "CIDR prefix must be 0–32" };
  }

  const ip = parseIpv4(ipRaw);
  if (ip === null) {
    return { ok: false, error: "CIDR IP must be a valid IPv4 address" };
  }

  const mask = prefix === 0 ? 0 : (0xffffffff << (32 - prefix)) >>> 0;
  const network = (ip & mask) >>> 0;
  const size = prefix === 32 ? 1 : 2 ** (32 - prefix);

  if (size > maxHosts) {
    return { ok: false, error: `CIDR range too large (>${maxHosts} hosts)` };
  }

  const first = network;
  const last = (network + size - 1) >>> 0;

  const hosts: string[] = [];
  const skipNetworkBroadcast = prefix <= 30 && size >= 4;

  for (let addr = first; addr <= last; addr++) {
    const isNetwork = addr === first;
    const isBroadcast = addr === last;
    if (skipNetworkBroadcast && (isNetwork || isBroadcast)) {
      continue;
    }
    hosts.push(formatIpv4(addr));
  }

  return { ok: true, cidr: `${formatIpv4(network)}/${prefix}`, hosts };
}

function parseIpv4(raw: string): number | null {
  const parts = raw.trim().split(".");
  if (parts.length !== 4) {
    return null;
  }
  const nums = parts.map((p) => Number(p));
  if (nums.some((n) => !Number.isInteger(n) || n < 0 || n > 255)) {
    return null;
  }
  return ((nums[0] << 24) | (nums[1] << 16) | (nums[2] << 8) | nums[3]) >>> 0;
}

function formatIpv4(value: number): string {
  const a = (value >>> 24) & 255;
  const b = (value >>> 16) & 255;
  const c = (value >>> 8) & 255;
  const d = value & 255;
  return `${a}.${b}.${c}.${d}`;
}
