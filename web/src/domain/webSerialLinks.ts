import type { WebSerialJsonlTransport } from "./hardwareConsole";

const EVENT_NAME = "isohub-web-serial-link";

export type WebSerialDeviceLink = {
  deviceId: string;
  transport: WebSerialJsonlTransport;
};

const transports = new Map<string, WebSerialJsonlTransport>();

function publishWebSerialDeviceLink(link: WebSerialDeviceLink): void {
  if (typeof window === "undefined") {
    return;
  }
  const previous = transports.get(link.deviceId);
  if (previous && previous !== link.transport) {
    void previous.disconnect().catch(() => undefined);
  }
  transports.set(link.deviceId, link.transport);
  window.dispatchEvent(new CustomEvent(EVENT_NAME, { detail: link }));
}

export function getWebSerialDeviceTransport(
  deviceId: string,
): WebSerialJsonlTransport | null {
  return transports.get(deviceId) ?? null;
}

export function announceWebSerialDeviceLink(link: WebSerialDeviceLink): void {
  publishWebSerialDeviceLink(link);
}

export function forgetWebSerialDeviceTransport(deviceId: string): void {
  void disconnectWebSerialDeviceTransport(deviceId).catch(() => undefined);
}

export async function disconnectWebSerialDeviceTransport(
  deviceId: string,
): Promise<void> {
  const transport = transports.get(deviceId);
  transports.delete(deviceId);
  if (transport) {
    await transport.disconnect();
  }
}

export function setWebSerialDeviceTransport(
  deviceId: string,
  transport: WebSerialJsonlTransport,
): void {
  publishWebSerialDeviceLink({ deviceId, transport });
}

export function subscribeWebSerialDeviceLinks(
  callback: (link: WebSerialDeviceLink) => void,
): () => void {
  if (typeof window === "undefined") {
    return () => {};
  }
  const onEvent = (event: Event) => {
    const detail = (event as CustomEvent<WebSerialDeviceLink>).detail;
    if (!detail?.deviceId || !detail.transport) {
      return;
    }
    callback(detail);
  };
  window.addEventListener(EVENT_NAME, onEvent);
  return () => window.removeEventListener(EVENT_NAME, onEvent);
}
