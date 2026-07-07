import type { CompanionBridge } from "../../domain/companionBridge";
import type { DeviceInfoResponse } from "../../domain/deviceApi";
import {
  nextJsonlRequestId,
  type SerialPortInfo,
  sendLocalUsbJsonlRequest,
  type WebSerialJsonlTransport,
} from "../../domain/hardwareConsole";

export type UsbInfoEnvelope = {
  ok?: boolean;
  result?: {
    device?: UsbDeviceInfo;
  };
  error?: { message?: string };
};

export type UsbDeviceInfo = {
  device_id?: string;
  hostname?: string;
  fqdn?: string;
  mac?: string;
  firmware?: { name?: string; version?: string };
  wifi?: { ipv4?: string | null };
};

export type UsbLogEntry = {
  id: number;
  tone: "info" | "success" | "warning" | "error";
  message: string;
};

export function parseUsbInfoEnvelope(
  value: unknown,
): { ok: true; device: UsbDeviceInfo } | { ok: false; error: string } {
  if (!value || typeof value !== "object") {
    return { ok: false, error: "USB device returned an invalid response." };
  }
  const envelope = value as UsbInfoEnvelope;
  if (envelope.ok === false) {
    return {
      ok: false,
      error: envelope.error?.message ?? "USB device rejected info request.",
    };
  }
  const device = envelope.result?.device;
  if (!device || typeof device !== "object") {
    return { ok: false, error: "USB device info response is missing device." };
  }
  return { ok: true, device };
}

export function InlineAddError({ message }: { message: string }) {
  return (
    <div
      className="mt-4 rounded-[12px] border border-[var(--error)] bg-[var(--panel)] px-4 py-3 text-[12px] font-semibold text-[var(--error)]"
      role="alert"
    >
      {message}
    </div>
  );
}

export function hydrateInitialUsbLog(
  entries: Array<Omit<UsbLogEntry, "id">> | undefined,
): UsbLogEntry[] {
  return entries?.map((entry, index) => ({ ...entry, id: index + 1 })) ?? [];
}

export function isIsolaRailDeviceInfo(device: UsbDeviceInfo): boolean {
  return (
    device.firmware?.name === "isolarail" ||
    device.hostname?.startsWith("isolarail-") ||
    device.fqdn?.startsWith("isolarail-") ||
    device.device_id !== undefined
  );
}

export function usbInfoMatchesHttpInfo(
  usbDevice: UsbDeviceInfo,
  usbDeviceId: string,
  httpInfo: DeviceInfoResponse,
): boolean {
  const httpDevice = httpInfo.device;
  const httpDeviceId = normalizeDeviceId(httpDevice.device_id);
  if (httpDeviceId && httpDeviceId === usbDeviceId) {
    return true;
  }
  const usbMac = normalizeMac(usbDevice.mac);
  const httpMac = normalizeMac(httpDevice.mac);
  return Boolean(usbMac && httpMac && usbMac === httpMac);
}

export function normalizeDeviceId(value: string | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

export function normalizeMac(value: string | null | undefined): string | null {
  const hex = value?.replace(/[^a-fA-F0-9]/g, "").toLowerCase();
  return hex && hex.length >= 6 ? hex : null;
}

export function shortIdFromMac(
  value: string | null | undefined,
): string | null {
  if (!value) {
    return null;
  }
  const hex = value.replace(/[^a-fA-F0-9]/g, "").toLowerCase();
  if (hex.length < 6) {
    return null;
  }
  return hex.slice(-6);
}

export function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

export async function readLocalUsbInfo(
  agent: CompanionBridge,
  port: SerialPortInfo,
  onLog: (message: string, tone?: UsbLogEntry["tone"]) => void,
): Promise<unknown> {
  let lastError: unknown;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      onLog(
        `Sending info request over Local USB (attempt ${attempt + 1}/3)...`,
      );
      return await sendLocalUsbJsonlRequest(agent, port.path, {
        id: nextJsonlRequestId(),
        method: "info",
        timeoutMs: 1_500,
      });
    } catch (err) {
      lastError = err;
      onLog(
        err instanceof Error
          ? `Local USB info attempt failed: ${err.message}`
          : "Local USB info attempt failed.",
        "warning",
      );
      await delay(250 + attempt * 250);
    }
  }
  throw lastError instanceof Error
    ? lastError
    : new Error("USB info request failed.");
}

export async function readWebSerialInfo(
  transport: WebSerialJsonlTransport,
  onLog: (message: string, tone?: UsbLogEntry["tone"]) => void,
): Promise<unknown> {
  let lastError: unknown;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      onLog(
        `Sending info request over Web Serial (attempt ${attempt + 1}/3)...`,
      );
      return await transport.request({
        id: nextJsonlRequestId(),
        method: "info",
        timeoutMs: 2_500 + attempt * 1_000,
      });
    } catch (err) {
      lastError = err;
      onLog(
        err instanceof Error
          ? `Web Serial info attempt failed: ${err.message}`
          : "Web Serial info attempt failed.",
        "warning",
      );
      await delay(250 + attempt * 250);
    }
  }
  throw lastError instanceof Error
    ? lastError
    : new Error("Web Serial info request failed.");
}
