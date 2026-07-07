const STORAGE_KEY = "isolarail.local_usb_links";
const EVENT_NAME = "isolarail-local-usb-link";

export type LocalUsbDeviceLink = {
  deviceId: string;
  portPath: string;
};

function readLinks(): Record<string, string> {
  if (typeof window === "undefined") {
    return {};
  }
  const raw = window.sessionStorage.getItem(STORAGE_KEY);
  if (!raw) {
    return {};
  }
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") {
      return {};
    }
    const links: Record<string, string> = {};
    for (const [deviceId, portPath] of Object.entries(parsed)) {
      if (typeof portPath === "string" && portPath.length > 0) {
        links[deviceId] = portPath;
      }
    }
    return links;
  } catch {
    return {};
  }
}

export function getLocalUsbDeviceLink(deviceId: string): string | null {
  return readLinks()[deviceId] ?? null;
}

export function announceLocalUsbDeviceLink(link: LocalUsbDeviceLink): void {
  if (typeof window === "undefined") {
    return;
  }
  const links = readLinks();
  links[link.deviceId] = link.portPath;
  window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(links));
  window.dispatchEvent(new CustomEvent(EVENT_NAME, { detail: link }));
}

export function forgetLocalUsbDeviceLink(deviceId: string): void {
  if (typeof window === "undefined") {
    return;
  }
  const links = readLinks();
  delete links[deviceId];
  window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(links));
}

export function subscribeLocalUsbDeviceLinks(
  callback: (link: LocalUsbDeviceLink) => void,
): () => void {
  if (typeof window === "undefined") {
    return () => {};
  }
  const onEvent = (event: Event) => {
    const detail = (event as CustomEvent<LocalUsbDeviceLink>).detail;
    if (!detail?.deviceId || !detail.portPath) {
      return;
    }
    callback(detail);
  };
  window.addEventListener(EVENT_NAME, onEvent);
  return () => window.removeEventListener(EVENT_NAME, onEvent);
}
