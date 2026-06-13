export type CompanionBridgeBootstrap = {
  token: string;
  agentBaseUrl: string;
  app: { name: string; version: string; mode: string };
};

export type CompanionBridge = {
  token: string;
  agentBaseUrl: string;
};

const LOCAL_USB_PORT_START = 51200;
const LOCAL_USB_PORT_END = 51299;

export function resolveAgentBaseUrl(
  bootstrapUrl: string,
  payloadAgentBaseUrl: string,
  currentLocationHref = currentBrowserHref(),
): string {
  if (!currentLocationHref) {
    return payloadAgentBaseUrl;
  }

  try {
    const current = new URL(currentLocationHref);
    const bootstrap = new URL(bootstrapUrl, current);
    if (bootstrap.origin === current.origin) {
      return current.origin;
    }
  } catch {
    return payloadAgentBaseUrl;
  }

  return payloadAgentBaseUrl;
}

export async function tryBootstrapCompanionBridge(): Promise<CompanionBridge | null> {
  const sameOrigin = await fetchCompanionBridgeBootstrap("/api/v1/bootstrap");
  if (sameOrigin) {
    return sameOrigin;
  }

  for (let port = LOCAL_USB_PORT_START; port <= LOCAL_USB_PORT_END; port += 1) {
    const agent = await fetchCompanionBridgeBootstrap(
      `http://127.0.0.1:${port}/api/v1/bootstrap`,
    );
    if (agent) {
      return agent;
    }
  }

  return null;
}

export async function agentFetch(
  agent: CompanionBridge,
  path: string,
  init?: RequestInit,
): Promise<Response> {
  const headers = new Headers(init?.headers);
  headers.set("Authorization", `Bearer ${agent.token}`);
  if (!headers.has("Content-Type") && init?.body) {
    headers.set("Content-Type", "application/json; charset=utf-8");
  }
  const url = path.startsWith("http")
    ? path
    : new URL(path, agent.agentBaseUrl).toString();
  return fetch(url, {
    ...init,
    headers,
    cache: "no-store",
  });
}

async function fetchCompanionBridgeBootstrap(
  url: string,
): Promise<CompanionBridge | null> {
  try {
    const res = await fetch(url, { cache: "no-store" });
    if (!res.ok) {
      return null;
    }
    const json = (await res.json()) as unknown;
    if (!json || typeof json !== "object") {
      return null;
    }
    const obj = json as Record<string, unknown>;
    const token = typeof obj.token === "string" ? obj.token : null;
    const payloadAgentBaseUrl =
      typeof obj.agentBaseUrl === "string" ? obj.agentBaseUrl : null;
    if (!token || !payloadAgentBaseUrl) {
      return null;
    }
    return {
      token,
      agentBaseUrl: resolveAgentBaseUrl(url, payloadAgentBaseUrl),
    };
  } catch {
    return null;
  }
}

function currentBrowserHref(): string | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }
  const location = window.location;
  return typeof location?.href === "string" ? location.href : undefined;
}
