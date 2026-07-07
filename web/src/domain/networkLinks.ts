const EVENT_NAME = "isolarail-network-link";

export type NetworkDeviceLink = {
  deviceId: string;
  baseUrl: string;
};

export function announceNetworkDeviceLink(link: NetworkDeviceLink): void {
  if (typeof window === "undefined") {
    return;
  }
  window.dispatchEvent(new CustomEvent(EVENT_NAME, { detail: link }));
}

export function subscribeNetworkDeviceLinks(
  callback: (link: NetworkDeviceLink) => void,
): () => void {
  if (typeof window === "undefined") {
    return () => {};
  }
  const onEvent = (event: Event) => {
    const detail = (event as CustomEvent<NetworkDeviceLink>).detail;
    if (!detail?.deviceId || !detail.baseUrl) {
      return;
    }
    callback(detail);
  };
  window.addEventListener(EVENT_NAME, onEvent);
  return () => window.removeEventListener(EVENT_NAME, onEvent);
}
