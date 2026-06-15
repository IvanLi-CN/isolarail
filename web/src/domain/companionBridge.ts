export type CompanionBridgeBootstrap = {
  token: string;
  agentBaseUrl: string;
  app: { name: string; version: string; mode: string };
};

export type CompanionBridge = {
  token: string;
  agentBaseUrl: string;
};

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
    const payload = new URL(payloadAgentBaseUrl, current);
    if (bootstrap.origin === current.origin) {
      const hasPathPrefix = payload.pathname !== "/";
      return hasPathPrefix ? payload.toString() : current.origin;
    }
  } catch {
    return payloadAgentBaseUrl;
  }

  return payloadAgentBaseUrl;
}

export async function tryBootstrapCompanionBridge(): Promise<CompanionBridge | null> {
  for (const url of companionBootstrapUrls()) {
    const agent = await fetchCompanionBridgeBootstrap(url);
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

export function companionBootstrapUrls(): string[] {
  const explicitOrigins =
    (import.meta.env.VITE_ISOHUB_DEVD_ORIGINS as string | undefined) ?? "";
  const origins = explicitOrigins
    .split(",")
    .map((origin) => origin.trim())
    .filter((origin) => origin.length > 0);
  if (origins.length === 0) {
    return ["/api/v1/bootstrap"];
  }
  return origins.map((origin) =>
    new URL("/api/v1/bootstrap", ensureTrailingSlash(origin)).toString(),
  );
}

function ensureTrailingSlash(value: string): string {
  return value.endsWith("/") ? value : `${value}/`;
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
