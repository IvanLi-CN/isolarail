import { describe, expect, test } from "bun:test";

import {
  companionBootstrapUrls,
  resolveAgentBaseUrl,
  tryBootstrapCompanionBridge,
} from "./companionBridge";

describe("resolveAgentBaseUrl", () => {
  test("keeps same-origin bootstrap requests on the current origin", () => {
    expect(
      resolveAgentBaseUrl(
        "/api/v1/bootstrap",
        "http://127.0.0.1:51200",
        "http://127.0.0.1:14890/devices",
      ),
    ).toBe("http://127.0.0.1:14890");
  });

  test("keeps absolute same-origin bootstrap requests on the current origin", () => {
    expect(
      resolveAgentBaseUrl(
        "http://127.0.0.1:14890/api/v1/bootstrap",
        "http://127.0.0.1:51200",
        "http://127.0.0.1:14890/about",
      ),
    ).toBe("http://127.0.0.1:14890");
  });

  test("preserves remote agent base urls for cross-origin bootstrap requests", () => {
    expect(
      resolveAgentBaseUrl(
        "http://127.0.0.1:51200/api/v1/bootstrap",
        "http://127.0.0.1:51200",
        "http://127.0.0.1:14890/devices",
      ),
    ).toBe("http://127.0.0.1:51200");
  });

  test("falls back to payload base url when current location is unavailable", () => {
    expect(
      resolveAgentBaseUrl(
        "/api/v1/bootstrap",
        "http://127.0.0.1:51200",
        undefined,
      ),
    ).toBe("http://127.0.0.1:51200");
  });
});

describe("companionBootstrapUrls", () => {
  test("uses only same-origin bootstrap when no explicit origins are configured", () => {
    expect(companionBootstrapUrls()).toEqual(["/api/v1/bootstrap"]);
  });

  test("uses explicitly configured origins in order", () => {
    import.meta.env.VITE_ISOHUB_DEVD_ORIGINS =
      "http://isohub-devd.local:51200, http://127.0.0.1:51200";

    expect(companionBootstrapUrls()).toEqual([
      "http://isohub-devd.local:51200/api/v1/bootstrap",
      "http://127.0.0.1:51200/api/v1/bootstrap",
    ]);

    import.meta.env.VITE_ISOHUB_DEVD_ORIGINS = "";
  });
});

describe("tryBootstrapCompanionBridge", () => {
  test("tries explicit mDNS origin before fallback origin without scanning ports", async () => {
    import.meta.env.VITE_ISOHUB_DEVD_ORIGINS =
      "http://isohub-devd.local:51200,http://127.0.0.1:51200";
    const calls: string[] = [];
    const originalFetch = globalThis.fetch;
    globalThis.fetch = (async (input: RequestInfo | URL) => {
      const url = input.toString();
      calls.push(url);
      if (url === "http://isohub-devd.local:51200/api/v1/bootstrap") {
        return new Response("", { status: 503 });
      }
      return Response.json({
        token: "test-token",
        agentBaseUrl: "http://127.0.0.1:51200",
        app: { name: "isohub-devd", version: "0.1.0", mode: "web" },
      });
    }) as typeof fetch;

    try {
      const bridge = await tryBootstrapCompanionBridge();

      expect(bridge?.agentBaseUrl).toBe("http://127.0.0.1:51200");
      expect(calls).toEqual([
        "http://isohub-devd.local:51200/api/v1/bootstrap",
        "http://127.0.0.1:51200/api/v1/bootstrap",
      ]);
    } finally {
      globalThis.fetch = originalFetch;
      import.meta.env.VITE_ISOHUB_DEVD_ORIGINS = "";
    }
  });
});
