import { describe, expect, test } from "bun:test";

import { resolveAgentBaseUrl } from "./companionBridge";

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
