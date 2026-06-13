import { agentFetch, type CompanionBridge } from "./companionBridge";

export type JsonlRequest = {
  id: number;
  method: string;
  params?: Record<string, unknown>;
  timeoutMs?: number;
};

export type SerialPortInfo = {
  path: string;
  label: string;
  vendorId?: number | null;
  productId?: number | null;
  serialNumber?: string | null;
  manufacturer?: string | null;
  product?: string | null;
};

const ESPRESSIF_USB_VENDOR_ID = 0x303a;
const ESP32_USB_SERIAL_JTAG_PRODUCT_ID = 0x1001;
export const DEFAULT_LOCAL_USB_FLASH_ADDRESS = 0x10000;
const DEFAULT_JSONL_TIMEOUT_MS = 5_000;
const LOCAL_USB_BUSY_RETRIES = 5;
let jsonlRequestSeq = 1;
const localUsbRequestQueues: Record<string, Promise<void>> = {};

export class LocalUsbAgentHttpError extends Error {
  readonly status: number;
  readonly code: string;
  readonly retryable: boolean;

  constructor(
    message: string,
    status: number,
    code = "local_usb_error",
    retryable = false,
  ) {
    super(message);
    this.status = status;
    this.code = code;
    this.retryable = retryable;
  }
}

export type HardwareTransportKind = "web_serial" | "local_usb";

export type DeviceIdentityExpectation = {
  deviceId?: string | null;
  mac?: string | null;
};

export type FirmwareFlashProgress = {
  stage: "connecting" | "writing" | "done";
  message: string;
  written?: number;
  total?: number;
};

export type SerialActivityEntry = {
  id: string;
  channel: "web_serial";
  kind: "json" | "raw" | "defmt" | "ignored";
  summary: string;
  payload: string;
  requestId: string | null;
  timestampMs: number;
};

export type SerialLikePort = SerialPort & {
  readable: ReadableStream<Uint8Array> | null;
  writable: WritableStream<Uint8Array> | null;
  getInfo?: () => { usbVendorId?: number; usbProductId?: number };
};

type EsptoolLoaderWithInternals = {
  ESP_MEM_END: number;
  _appendArray(left: Uint8Array, right: Uint8Array): Uint8Array;
  _intToByteArray(value: number): Uint8Array;
  checkCommand(
    opDescription: string,
    op: number,
    data: Uint8Array,
    checksum?: number,
    responseDataLength?: number,
    timeout?: number,
  ): Promise<unknown>;
  memFinish(entrypoint: number): Promise<void>;
};

type EsptoolTransportWithSignals = {
  setDTR(state: boolean): Promise<void>;
  setRTS(state: boolean): Promise<void>;
};

declare global {
  type SerialPort = {
    readable: ReadableStream<Uint8Array> | null;
    writable: WritableStream<Uint8Array> | null;
    open(options: { baudRate: number }): Promise<void>;
    close(): Promise<void>;
  };

  interface Navigator {
    serial: {
      getPorts?: () => Promise<SerialPort[]>;
      requestPort(options?: unknown): Promise<SerialPort>;
    };
  }
}

export function isWebSerialSupported(): boolean {
  return typeof navigator !== "undefined" && "serial" in navigator;
}

export function nextJsonlRequestId(): number {
  const id = jsonlRequestSeq;
  jsonlRequestSeq = jsonlRequestSeq >= 999_999 ? 1 : jsonlRequestSeq + 1;
  return id;
}

export async function listLocalUsbSerialPorts(
  agent: CompanionBridge,
): Promise<SerialPortInfo[]> {
  const res = await agentFetch(agent, "/api/v1/devices/scan", {
    method: "POST",
  });
  if (!res.ok) {
    if (res.status === 404) {
      return listLegacyLocalUsbSerialPorts(agent);
    }
    throw new Error(`Local USB port list failed (${res.status})`);
  }
  const json = (await res.json()) as {
    devices?: Array<{
      usb?: {
        portPath?: string;
        label?: string;
        vendorId?: number | null;
        productId?: number | null;
        serialNumber?: string | null;
      };
    }>;
  };
  return Array.isArray(json.devices)
    ? json.devices
        .map((device) => device.usb)
        .filter((usb): usb is NonNullable<typeof usb> => Boolean(usb))
        .map((usb) => ({
          path: usb.portPath ?? "",
          label: usb.label ?? usb.portPath ?? "Local USB",
          vendorId: usb.vendorId,
          productId: usb.productId,
          serialNumber: usb.serialNumber,
        }))
        .filter((port) => port.path.length > 0)
    : [];
}

async function listLegacyLocalUsbSerialPorts(
  agent: CompanionBridge,
): Promise<SerialPortInfo[]> {
  const res = await agentFetch(agent, "/api/v1/serial/ports");
  if (!res.ok) {
    throw new Error(`Local USB port list failed (${res.status})`);
  }
  const json = (await res.json()) as { ports?: SerialPortInfo[] };
  return Array.isArray(json.ports) ? json.ports : [];
}

export function filterEsp32SerialPorts(
  ports: SerialPortInfo[],
): SerialPortInfo[] {
  const esp32Ports = ports
    .filter(isEsp32SerialPort)
    .sort(compareSerialPortsForConnect);
  return dedupeSerialDevicePairs(esp32Ports);
}

export function isEsp32SerialPort(port: SerialPortInfo): boolean {
  const path = port.path.toLowerCase();
  if (path.includes("bluetooth") || path.includes("debug-console")) {
    return false;
  }

  const manufacturer = (port.manufacturer ?? "").toLowerCase();
  const product = (port.product ?? port.label ?? "").toLowerCase();
  const vendorMatches = port.vendorId === ESPRESSIF_USB_VENDOR_ID;
  const serialJtagMatches =
    vendorMatches && port.productId === ESP32_USB_SERIAL_JTAG_PRODUCT_ID;
  if (serialJtagMatches) {
    return true;
  }

  const pathLooksLikeUsbSerial =
    path.includes("usbmodem") ||
    path.includes("usbserial") ||
    path.includes("ttyacm") ||
    /^com\d+$/i.test(port.path);
  const espressifTextMatches =
    manufacturer.includes("espressif") ||
    product.includes("esp32") ||
    product.includes("jtag/serial") ||
    product.includes("usb jtag");

  return pathLooksLikeUsbSerial && espressifTextMatches;
}

function compareSerialPortsForConnect(
  a: SerialPortInfo,
  b: SerialPortInfo,
): number {
  const aCu = isCuPort(a) ? 0 : 1;
  const bCu = isCuPort(b) ? 0 : 1;
  if (aCu !== bCu) {
    return aCu - bCu;
  }
  return a.path.localeCompare(b.path);
}

function dedupeSerialDevicePairs(ports: SerialPortInfo[]): SerialPortInfo[] {
  const seen = new Set<string>();
  const filtered: SerialPortInfo[] = [];
  for (const port of ports) {
    const key = port.serialNumber ?? pairedDeviceKey(port.path);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    filtered.push(port);
  }
  return filtered;
}

function isCuPort(port: SerialPortInfo): boolean {
  return port.path.startsWith("/dev/cu.");
}

function pairedDeviceKey(path: string): string {
  return path.replace("/dev/tty.", "/dev/cu.");
}

export async function sendLocalUsbJsonlRequest(
  agent: CompanionBridge,
  portPath: string,
  request: JsonlRequest,
): Promise<unknown> {
  const previous = localUsbRequestQueues[portPath] ?? Promise.resolve();
  let releaseQueue: () => void = () => undefined;
  const current = new Promise<void>((resolve) => {
    releaseQueue = resolve;
  });
  const queued = previous.catch(() => undefined).then(() => current);
  localUsbRequestQueues[portPath] = queued;
  await previous.catch(() => undefined);
  try {
    let lastError: Error | null = null;
    for (let attempt = 0; attempt <= LOCAL_USB_BUSY_RETRIES; attempt += 1) {
      try {
        return await sendLocalUsbJsonlRequestNow(agent, portPath, request);
      } catch (err) {
        const error =
          err instanceof Error ? err : new Error("Local USB request failed");
        lastError = error;
        if (!error.message.includes("serial port is busy")) {
          throw error;
        }
        if (attempt >= LOCAL_USB_BUSY_RETRIES) {
          throw error;
        }
        await delay(200 + attempt * 250);
      }
    }
    throw lastError ?? new Error("Local USB request failed");
  } finally {
    releaseQueue();
    if (localUsbRequestQueues[portPath] === queued) {
      delete localUsbRequestQueues[portPath];
    }
  }
}

export async function sendDevdLocalUsbJsonlRequest(
  agent: CompanionBridge,
  deviceId: string,
  request: JsonlRequest,
): Promise<unknown> {
  await ensureDevdLocalUsbDeviceRegistered(agent, deviceId);
  const endpoint = localUsbMethodEndpoint(deviceId, request);
  let lease: { lease_id: string } | null = null;
  try {
    if (request.method === "reboot") {
      lease = await createLocalUsbLease(agent, deviceId);
      endpoint.body = {
        ...(endpoint.body as object),
        lease_id: lease.lease_id,
      };
    }
    const res = await agentFetch(agent, endpoint.path, {
      method: endpoint.method,
      body: endpoint.body ? JSON.stringify(endpoint.body) : undefined,
    });
    const json = (await res.json().catch(() => null)) as {
      response?: unknown;
      error?: { code?: string; message?: string; retryable?: boolean };
    } | null;
    if (!res.ok) {
      throw new LocalUsbAgentHttpError(
        json?.error?.message ?? `Local USB request failed (${res.status})`,
        res.status,
        json?.error?.code,
        json?.error?.retryable,
      );
    }
    return json?.response ?? json;
  } finally {
    if (lease) {
      await releaseLocalUsbLease(agent, lease.lease_id);
    }
  }
}

async function ensureDevdLocalUsbDeviceRegistered(
  agent: CompanionBridge,
  deviceId: string,
): Promise<void> {
  const res = await agentFetch(agent, "/api/v1/devices/scan", {
    method: "POST",
  });
  const json = (await res.json().catch(() => null)) as {
    devices?: Array<{ id?: string }>;
    error?: { code?: string; message?: string; retryable?: boolean };
  } | null;
  if (!res.ok) {
    throw new LocalUsbAgentHttpError(
      json?.error?.message ?? "Local USB scan failed",
      res.status,
      json?.error?.code,
      json?.error?.retryable,
    );
  }
  if (!json?.devices?.some((device) => device.id === deviceId)) {
    throw new Error(`Local USB device is not available: ${deviceId}`);
  }
}

async function sendLocalUsbJsonlRequestNow(
  agent: CompanionBridge,
  portPath: string,
  request: JsonlRequest,
): Promise<unknown> {
  const deviceId = stableLocalUsbDeviceId(portPath);
  await ensureLocalUsbDeviceRegistered(agent, deviceId, portPath);
  const endpoint = localUsbMethodEndpoint(deviceId, request);
  let lease: { lease_id: string } | null = null;
  try {
    if (request.method === "reboot") {
      lease = await createLocalUsbLease(agent, deviceId);
      endpoint.body = {
        ...(endpoint.body as object),
        lease_id: lease.lease_id,
      };
    }
    const res = await agentFetch(agent, endpoint.path, {
      method: endpoint.method,
      body: endpoint.body ? JSON.stringify(endpoint.body) : undefined,
    });
    const json = (await res.json().catch(() => null)) as {
      response?: unknown;
      error?: { code?: string; message?: string; retryable?: boolean };
    } | null;
    if (!res.ok) {
      if (res.status === 404) {
        return legacyLocalUsbJsonlRequest(agent, portPath, request);
      }
      throw new LocalUsbAgentHttpError(
        json?.error?.message ?? `Local USB request failed (${res.status})`,
        res.status,
        json?.error?.code,
        json?.error?.retryable,
      );
    }
    return json?.response ?? json;
  } catch (err) {
    if (err instanceof LocalUsbAgentHttpError && err.status === 404) {
      return legacyLocalUsbJsonlRequest(agent, portPath, request);
    }
    throw err;
  } finally {
    if (lease) {
      await releaseLocalUsbLease(agent, lease.lease_id);
    }
  }
}

async function legacyLocalUsbJsonlRequest(
  agent: CompanionBridge,
  portPath: string,
  request: JsonlRequest,
): Promise<unknown> {
  const res = await agentFetch(agent, "/api/v1/serial/request", {
    method: "POST",
    body: JSON.stringify({
      portPath,
      request,
      timeoutMs: request.timeoutMs,
    }),
  });
  const json = (await res.json().catch(() => null)) as {
    response?: unknown;
    error?: { code?: string; message?: string; retryable?: boolean };
  } | null;
  if (!res.ok) {
    throw new LocalUsbAgentHttpError(
      json?.error?.message ?? `Local USB request failed (${res.status})`,
      res.status,
      json?.error?.code,
      json?.error?.retryable,
    );
  }
  return json?.response ?? json;
}

export function stableLocalUsbDeviceId(portPath: string): string {
  const sanitized = Array.from(portPath)
    .map((ch) => (/[a-zA-Z0-9]/.test(ch) ? ch : "-"))
    .join("");
  return `usb-${sanitized}`;
}

export function devdLocalUsbDeviceIdFromBaseUrl(
  baseUrl: string,
): string | null {
  const prefix = "isohub-devd://";
  if (!baseUrl.startsWith(prefix)) {
    return null;
  }
  const deviceId = baseUrl.slice(prefix.length).trim();
  return deviceId.length > 0 ? deviceId : null;
}

async function ensureLocalUsbDeviceRegistered(
  agent: CompanionBridge,
  deviceId: string,
  portPath: string,
): Promise<void> {
  const res = await agentFetch(agent, "/api/v1/devices/scan", {
    method: "POST",
  });
  const json = (await res.json().catch(() => null)) as {
    devices?: Array<{ id?: string; usb?: { portPath?: string } }>;
    error?: { code?: string; message?: string; retryable?: boolean };
  } | null;
  if (!res.ok) {
    if (res.status === 404) {
      const ports = await listLegacyLocalUsbSerialPorts(agent);
      if (ports.some((port) => port.path === portPath)) {
        return;
      }
      throw new Error(`Local USB device is not available: ${portPath}`);
    }
    throw new LocalUsbAgentHttpError(
      json?.error?.message ?? "Local USB scan failed",
      res.status,
      json?.error?.code,
      json?.error?.retryable,
    );
  }
  const registered = json?.devices?.some(
    (device) => device.id === deviceId || device.usb?.portPath === portPath,
  );
  if (!registered) {
    throw new Error(`Local USB device is not available: ${portPath}`);
  }
}

function localUsbMethodEndpoint(
  deviceId: string,
  request: JsonlRequest,
): { method: "GET" | "POST" | "PUT" | "DELETE"; path: string; body?: unknown } {
  const params = request.params ?? {};
  switch (request.method) {
    case "info":
      return { method: "GET", path: `/api/v1/devices/${deviceId}/status` };
    case "ports.get":
      return { method: "GET", path: `/api/v1/devices/${deviceId}/ports` };
    case "wifi.get":
      return { method: "GET", path: `/api/v1/devices/${deviceId}/wifi` };
    case "wifi.set":
      return {
        method: "POST",
        path: `/api/v1/devices/${deviceId}/wifi`,
        body: params,
      };
    case "wifi.clear":
      return { method: "DELETE", path: `/api/v1/devices/${deviceId}/wifi` };
    case "reboot":
      return {
        method: "POST",
        path: `/api/v1/devices/${deviceId}/reset`,
        body: {},
      };
    case "port.power_set": {
      const port = String(params.port ?? "");
      const enabled = params.enabled ? "1" : "0";
      return {
        method: "POST",
        path: `/api/v1/devices/${deviceId}/ports/${port}/power?enabled=${enabled}`,
      };
    }
    case "port.replug": {
      const port = String(params.port ?? "");
      return {
        method: "POST",
        path: `/api/v1/devices/${deviceId}/ports/${port}/replug`,
      };
    }
    default:
      throw new Error(`Unsupported Local USB method: ${request.method}`);
  }
}

async function createLocalUsbLease(
  agent: CompanionBridge,
  deviceId: string,
): Promise<{ lease_id: string }> {
  const res = await agentFetch(agent, "/api/v1/serial/lease", {
    method: "POST",
    body: JSON.stringify({ device_id: deviceId }),
  });
  const json = (await res.json().catch(() => null)) as {
    lease_id?: string;
    error?: { code?: string; message?: string; retryable?: boolean };
  } | null;
  if (!res.ok || typeof json?.lease_id !== "string") {
    throw new LocalUsbAgentHttpError(
      json?.error?.message ?? "Local USB lease failed",
      res.status,
      json?.error?.code,
      json?.error?.retryable,
    );
  }
  return { lease_id: json.lease_id };
}

async function releaseLocalUsbLease(
  agent: CompanionBridge,
  leaseId: string,
): Promise<void> {
  await agentFetch(agent, `/api/v1/serial/lease/${leaseId}`, {
    method: "DELETE",
  }).catch(() => undefined);
}

export async function flashWithLocalUsb(
  agent: CompanionBridge,
  portPath: string,
  file: File,
  address: number,
  expectedIdentity: DeviceIdentityExpectation,
): Promise<string> {
  if (address !== DEFAULT_LOCAL_USB_FLASH_ADDRESS) {
    throw new Error(
      "Local USB firmware flashing writes the app image at 0x10000.",
    );
  }
  const deviceId = stableLocalUsbDeviceId(portPath);
  const firmware = await fileToBase64(file);
  let lease: { lease_id: string } | null = null;
  try {
    await ensureLocalUsbDeviceRegistered(agent, deviceId, portPath);
    lease = await createLocalUsbLease(agent, deviceId);
    const res = await agentFetch(
      agent,
      `/api/v1/devices/${deviceId}/flash-upload`,
      {
        method: "POST",
        body: JSON.stringify({
          address,
          fileName: file.name,
          fileBase64: firmware,
          expectedIdentity: {
            deviceId: expectedIdentity.deviceId ?? undefined,
            mac: expectedIdentity.mac ?? undefined,
          },
          leaseId: lease.lease_id,
        }),
      },
    );
    const json = (await res.json()) as {
      ok?: boolean;
      log?: string;
      error?: { code?: string; message?: string; retryable?: boolean };
    };
    if (!res.ok || !json.ok) {
      if (res.status === 404) {
        return legacyFlashWithLocalUsb(
          agent,
          portPath,
          file.name,
          address,
          firmware,
          expectedIdentity,
        );
      }
      throw new LocalUsbAgentHttpError(
        json.error?.message ||
          json.log ||
          `Local USB flash failed (${res.status})`,
        res.status,
        json.error?.code,
        json.error?.retryable,
      );
    }
    return json.log ?? "";
  } catch (err) {
    if (err instanceof LocalUsbAgentHttpError && err.status === 404) {
      return legacyFlashWithLocalUsb(
        agent,
        portPath,
        file.name,
        address,
        firmware,
        expectedIdentity,
      );
    }
    throw err;
  } finally {
    if (lease) {
      await releaseLocalUsbLease(agent, lease.lease_id);
    }
  }
}

async function legacyFlashWithLocalUsb(
  agent: CompanionBridge,
  portPath: string,
  fileName: string,
  address: number,
  fileBase64: string,
  expectedIdentity: DeviceIdentityExpectation,
): Promise<string> {
  const res = await agentFetch(agent, "/api/v1/firmware/flash", {
    method: "POST",
    body: JSON.stringify({
      portPath,
      address,
      fileName,
      fileBase64,
      expectedIdentity: {
        deviceId: expectedIdentity.deviceId ?? undefined,
        mac: expectedIdentity.mac ?? undefined,
      },
    }),
  });
  const json = (await res.json()) as {
    ok?: boolean;
    log?: string;
    error?: { code?: string; message?: string; retryable?: boolean };
  };
  if (!res.ok || !json.ok) {
    throw new LocalUsbAgentHttpError(
      json.error?.message ||
        json.log ||
        `Local USB flash failed (${res.status})`,
      res.status,
      json.error?.code,
      json.error?.retryable,
    );
  }
  return json.log ?? "";
}

export class WebSerialJsonlTransport {
  private port: SerialLikePort | null = null;
  private reader: ReadableStreamDefaultReader<Uint8Array> | null = null;
  private writer: WritableStreamDefaultWriter<Uint8Array> | null = null;
  private decoder = new TextDecoder();
  private requestQueue: Promise<void> = Promise.resolve();
  private readBuffer: number[] = [];
  private defmtBuffer: number[] = [];
  private defmtInFrame = false;
  private activityListeners = new Set<(entry: SerialActivityEntry) => void>();
  private pending = new Map<
    string,
    {
      resolve: (value: unknown) => void;
      reject: (err: Error) => void;
      timeoutId: ReturnType<typeof globalThis.setTimeout>;
    }
  >();

  async connect(): Promise<void> {
    if (!isWebSerialSupported()) {
      throw new Error("Web Serial is not supported by this browser");
    }
    const grantedPort = await findGrantedWebSerialPort();
    if (grantedPort) {
      try {
        await this.connectToPort(grantedPort);
        return;
      } catch {
        await this.disconnect().catch(() => undefined);
      }
    }
    await this.connectToPort(await requestWebSerialPort());
  }

  async connectWithPicker(): Promise<void> {
    if (!isWebSerialSupported()) {
      throw new Error("Web Serial is not supported by this browser");
    }
    await this.connectToPort(await requestWebSerialPort());
  }

  async connectGranted(): Promise<void> {
    if (!isWebSerialSupported()) {
      throw new Error("Web Serial is not supported by this browser");
    }
    const port = await findGrantedWebSerialPort();
    if (!port) {
      throw new Error("No authorized ESP32-S3 Web Serial port is available.");
    }
    await this.connectToPort(port);
  }

  async connectToPort(port: SerialLikePort): Promise<void> {
    await port.open({ baudRate: 115200 });
    this.reader = port.readable?.getReader() ?? null;
    this.writer = port.writable?.getWriter() ?? null;
    this.decoder = new TextDecoder();
    this.readBuffer = [];
    this.defmtBuffer = [];
    this.defmtInFrame = false;
    this.port = port;
    void this.readSerialLoop();
  }

  subscribeActivity(
    listener: (entry: SerialActivityEntry) => void,
  ): () => void {
    this.activityListeners.add(listener);
    return () => {
      this.activityListeners.delete(listener);
    };
  }

  async takePortForExclusiveUse(): Promise<SerialLikePort> {
    const port = this.port;
    if (!port) {
      throw new Error("Web Serial transport is not connected");
    }
    await this.disconnect();
    return port;
  }

  async request(request: JsonlRequest): Promise<unknown> {
    const run = this.requestQueue.then(() => this.performRequest(request));
    this.requestQueue = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
  }

  private async performRequest(request: JsonlRequest): Promise<unknown> {
    if (!this.reader || !this.writer) {
      throw new Error("Web Serial transport is not connected");
    }
    const payload = `${JSON.stringify(request)}\n`;
    this.emitActivity({
      kind: "json",
      summary: `json request ${request.method}`,
      payload: payload.trim(),
      requestId: String(request.id),
    });
    const response = this.waitForResponse(request);
    try {
      await this.writer.write(new TextEncoder().encode(payload));
      return await response;
    } catch (err) {
      this.clearPendingRequest(request.id);
      throw err;
    }
  }

  private waitForResponse(request: JsonlRequest): Promise<unknown> {
    const key = String(request.id);
    this.clearPendingRequest(request.id);
    return new Promise((resolve, reject) => {
      const timeoutId = globalThis.setTimeout(() => {
        this.pending.delete(key);
        reject(
          new Error(
            "No IsoHub JSONL response received from this serial device.",
          ),
        );
      }, request.timeoutMs ?? DEFAULT_JSONL_TIMEOUT_MS);
      this.pending.set(key, { resolve, reject, timeoutId });
    });
  }

  private clearPendingRequest(requestId: number): void {
    const key = String(requestId);
    const pending = this.pending.get(key);
    if (!pending) {
      return;
    }
    globalThis.clearTimeout(pending.timeoutId);
    this.pending.delete(key);
  }

  private async readSerialLoop(): Promise<void> {
    const reader = this.reader;
    if (!reader) {
      return;
    }
    try {
      for (;;) {
        const chunk = await reader.read();
        if (chunk.done) {
          throw new Error("Serial stream closed before a JSONL response");
        }
        this.consumeMonitorBytes(chunk.value);
      }
    } catch (err) {
      if (this.reader === reader) {
        this.rejectPending(
          err instanceof Error ? err : new Error("Web Serial read failed"),
        );
      }
    }
  }

  private consumeMonitorBytes(bytes: ArrayLike<number>): void {
    for (let index = 0; index < bytes.length; index += 1) {
      this.defmtBuffer.push(bytes[index]);
    }
    this.drainDefmtBuffer();
  }

  private drainDefmtBuffer(): void {
    for (;;) {
      if (this.defmtInFrame) {
        const end = findDefmtFrameEnd(this.defmtBuffer);
        if (end === -1) {
          return;
        }
        const frame = this.defmtBuffer.slice(0, end);
        this.defmtBuffer.splice(0, end + 1);
        this.defmtInFrame = false;
        this.emitActivity({
          kind: "defmt",
          summary: "defmt/raw binary frame",
          payload: hexPreview(frame),
          requestId: null,
        });
        continue;
      }

      const start = findDefmtFrameStart(this.defmtBuffer);
      if (start === -1) {
        const keep = this.defmtBuffer.at(-1) === 0xff ? 1 : 0;
        const raw = this.defmtBuffer.splice(0, this.defmtBuffer.length - keep);
        if (raw.length > 0) {
          this.consumeLineBytesStream(raw);
        }
        return;
      }

      const raw = this.defmtBuffer.splice(0, start);
      if (raw.length > 0) {
        this.consumeLineBytesStream(raw);
      }
      this.defmtBuffer.splice(0, 2);
      this.defmtInFrame = true;
    }
  }

  private consumeLineBytesStream(bytes: ArrayLike<number>): void {
    for (let index = 0; index < bytes.length; index += 1) {
      const byte = bytes[index];
      if (byte === 10) {
        this.consumeLineBytes(this.readBuffer);
        this.readBuffer = [];
        continue;
      }
      if (byte === 13) {
        continue;
      }
      this.readBuffer.push(byte);
      if (this.readBuffer.length > 16_384) {
        this.emitActivity({
          kind: "ignored",
          summary: "cdc line exceeded 16 KiB",
          payload: hexPreview(this.readBuffer),
          requestId: null,
        });
        this.readBuffer = [];
      }
    }
  }

  private consumeLineBytes(lineBytes: number[]): void {
    if (lineBytes.length === 0) {
      return;
    }
    const candidate = extractJsonCandidateFromBytes(lineBytes, this.decoder);
    if (!candidate) {
      const decoded = decodeLineBytes(lineBytes);
      if (decoded === null) {
        this.emitActivity({
          kind: "defmt",
          summary: "defmt/raw binary frame",
          payload: hexPreview(lineBytes),
          requestId: null,
        });
        return;
      }
      if (decoded.length > 0) {
        this.emitActivity({
          kind: "raw",
          summary: "raw cdc line",
          payload: decoded,
          requestId: null,
        });
      }
      return;
    }
    try {
      const parsed = JSON.parse(candidate) as unknown;
      this.emitActivity({
        kind: "json",
        summary: describeJsonActivity(parsed),
        payload: candidate,
        requestId: jsonlResponseId(parsed),
      });
      const id = jsonlResponseId(parsed);
      if (id === null) {
        return;
      }
      const pending = this.pending.get(id);
      if (!pending) {
        return;
      }
      globalThis.clearTimeout(pending.timeoutId);
      this.pending.delete(id);
      pending.resolve(parsed);
    } catch {
      // Ignore boot logs or non-IsoHub serial output until a JSONL frame appears.
    }
  }

  private rejectPending(err: Error): void {
    for (const pending of this.pending.values()) {
      globalThis.clearTimeout(pending.timeoutId);
      pending.reject(err);
    }
    this.pending.clear();
  }

  async disconnect(): Promise<void> {
    const reader = this.reader;
    const writer = this.writer;
    const port = this.port;

    this.reader = null;
    this.writer = null;
    this.port = null;
    this.decoder = new TextDecoder();
    this.readBuffer = [];
    this.defmtBuffer = [];
    this.defmtInFrame = false;
    this.requestQueue = Promise.resolve();
    this.rejectPending(new Error("Web Serial transport disconnected"));

    try {
      await reader?.cancel();
    } catch {
      // Ignore cancellation errors while tearing down the serial stream.
    }

    try {
      reader?.releaseLock();
    } catch {
      // The reader may already be released after cancellation.
    }

    try {
      writer?.releaseLock();
    } catch {
      // The writer may already be released if the port was closed externally.
    }

    try {
      await port?.close();
    } catch (err) {
      if (
        !(err instanceof DOMException) ||
        !err.message.includes("already closed")
      ) {
        throw err;
      }
    }
  }

  private emitActivity(
    entry: Omit<SerialActivityEntry, "id" | "channel" | "timestampMs">,
  ): void {
    const next: SerialActivityEntry = {
      id: `serial-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`,
      channel: "web_serial",
      timestampMs: Date.now(),
      ...entry,
    };
    for (const listener of this.activityListeners) {
      listener(next);
    }
  }
}

function extractJsonCandidateFromBytes(
  lineBytes: ArrayLike<number>,
  decoder: TextDecoder,
): string | null {
  const bytes = Uint8Array.from(lineBytes);
  for (let start = 0; start < bytes.length; start += 1) {
    if (bytes[start] !== 0x7b) {
      continue;
    }
    for (let end = bytes.length; end > start; end -= 1) {
      if (bytes[end - 1] !== 0x7d) {
        continue;
      }
      try {
        return decoder.decode(bytes.slice(start, end)).trim();
      } catch {}
    }
  }
  return null;
}

function decodeLineBytes(lineBytes: ArrayLike<number>): string | null {
  try {
    return new TextDecoder("utf-8", { fatal: true })
      .decode(Uint8Array.from(lineBytes))
      .trim();
  } catch {
    return null;
  }
}

function describeJsonActivity(parsed: unknown): string {
  if (!parsed || typeof parsed !== "object") {
    return "json frame";
  }
  const frame = parsed as Record<string, unknown>;
  const type = typeof frame.type === "string" ? frame.type : null;
  if (type === "log") {
    const level = typeof frame.level === "string" ? frame.level : "info";
    const target =
      typeof frame.target === "string" ? frame.target : "usb_jsonl";
    const message =
      typeof frame.message === "string" ? frame.message : "USB CDC log";
    return `json log ${level} ${target}: ${message}`;
  }
  const method = typeof frame.method === "string" ? frame.method : null;
  if (method) {
    return `json request ${method}`;
  }
  const id = jsonlResponseId(parsed);
  if (id) {
    return `json response ${id}`;
  }
  return type ? `json ${type}` : "json frame";
}

function hexPreview(bytes: ArrayLike<number>): string {
  const slice = Array.from(bytes).slice(0, 96);
  const preview = slice
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join(" ");
  return Array.from(bytes).length > 96 ? `${preview} …` : preview;
}

function findDefmtFrameStart(buffer: number[]): number {
  for (let index = 0; index < buffer.length - 1; index += 1) {
    if (buffer[index] === 0xff && buffer[index + 1] === 0x00) {
      return index;
    }
  }
  return -1;
}

function findDefmtFrameEnd(buffer: number[]): number {
  for (let index = 0; index < buffer.length; index += 1) {
    if (buffer[index] === 0x00) {
      return index;
    }
  }
  return -1;
}

async function requestWebSerialPort(): Promise<SerialLikePort> {
  return (await navigator.serial.requestPort({
    filters: [
      {
        usbVendorId: ESPRESSIF_USB_VENDOR_ID,
        usbProductId: ESP32_USB_SERIAL_JTAG_PRODUCT_ID,
      },
      { usbVendorId: ESPRESSIF_USB_VENDOR_ID },
    ],
  })) as SerialLikePort;
}

async function findGrantedWebSerialPort(): Promise<SerialLikePort | null> {
  const ports = ((await navigator.serial.getPorts?.()) ??
    []) as SerialLikePort[];
  return ports.find(isEspressifWebSerialPort) ?? null;
}

function isEspressifWebSerialPort(port: SerialLikePort): boolean {
  const info = port.getInfo?.();
  if (!info) {
    return false;
  }
  return (
    info.usbVendorId === ESPRESSIF_USB_VENDOR_ID &&
    (!info.usbProductId ||
      info.usbProductId === ESP32_USB_SERIAL_JTAG_PRODUCT_ID)
  );
}

function jsonlResponseId(value: unknown): string | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const id = (value as { id?: unknown }).id;
  return id === undefined || id === null ? null : String(id);
}

export async function flashWithWebSerial(
  port: SerialLikePort,
  file: File,
  address: number,
  onProgress: (progress: FirmwareFlashProgress) => void,
): Promise<void> {
  if (!isWebSerialSupported()) {
    throw new Error("Web Serial is not supported by this browser");
  }

  const { ESPLoader, Transport } = await import("esptool-js");
  const transport = new Transport(port, true);
  const terminal = {
    clean() {},
    writeLine(data: string) {
      onProgress({ stage: "connecting", message: data });
    },
    write(data: string) {
      onProgress({ stage: "connecting", message: data });
    },
  };
  const loader = new ESPLoader({
    transport,
    baudrate: 115200,
    terminal,
    debugLogging: false,
  });
  patchEsp32S3UsbJtagStubStart(loader as EsptoolLoaderWithInternals);

  onProgress({ stage: "connecting", message: "Connecting to bootloader" });
  try {
    await loader.main("usb_reset");
    const data = new Uint8Array(await file.arrayBuffer());
    await loader.writeFlash({
      fileArray: [{ data, address }],
      flashMode: "dio",
      flashFreq: "40m",
      flashSize: "4MB",
      eraseAll: false,
      compress: true,
      reportProgress: (_fileIndex, written, total) => {
        onProgress({
          stage: "writing",
          message: "Writing firmware",
          written,
          total,
        });
      },
    });
    await loader.after("hard_reset");
    await resetEsp32S3UsbJtagToApp(transport as EsptoolTransportWithSignals);
    onProgress({ stage: "done", message: "Firmware written" });
  } finally {
    await transport.disconnect().catch(() => undefined);
  }
}

function patchEsp32S3UsbJtagStubStart(loader: EsptoolLoaderWithInternals) {
  loader.memFinish = async (entrypoint: number) => {
    const isEntry = entrypoint === 0 ? 1 : 0;
    const packet = loader._appendArray(
      loader._intToByteArray(isEntry),
      loader._intToByteArray(entrypoint),
    );
    await loader.checkCommand(
      "leave RAM download mode",
      loader.ESP_MEM_END,
      packet,
      undefined,
      undefined,
      2_000,
    );
  };
}

async function resetEsp32S3UsbJtagToApp(
  transport: EsptoolTransportWithSignals,
): Promise<void> {
  await transport.setDTR(false);
  await transport.setRTS(false);
  await delay(100);
  await transport.setDTR(true);
  await transport.setRTS(false);
  await delay(100);
  await transport.setDTR(false);
  await transport.setRTS(true);
  await delay(100);
  await transport.setDTR(false);
  await transport.setRTS(false);
  await delay(500);
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => globalThis.setTimeout(resolve, ms));
}

async function fileToBase64(file: File): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  let binary = "";
  const chunkSize = 0x2000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }
  return btoa(binary);
}
