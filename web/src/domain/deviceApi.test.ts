import { afterEach, describe, expect, test } from "bun:test";

import { shouldUsePna } from "./deviceApi";

const originalWindow = globalThis.window;

afterEach(() => {
  globalThis.window = originalWindow;
});

function setSecureContext(value: boolean) {
  globalThis.window = { isSecureContext: value } as Window & typeof globalThis;
}

describe("shouldUsePna", () => {
  test("does not use PNA outside secure contexts", () => {
    setSecureContext(false);

    expect(shouldUsePna("http://192.168.1.10")).toBe(false);
  });

  test("does not use PNA for loopback hosts", () => {
    setSecureContext(true);

    expect(shouldUsePna("http://localhost:51200")).toBe(false);
    expect(shouldUsePna("http://127.0.0.1:51200")).toBe(false);
    expect(shouldUsePna("http://[::1]:51200")).toBe(false);
  });

  test("uses PNA for non-loopback HTTP targets from secure contexts", () => {
    setSecureContext(true);

    expect(shouldUsePna("http://192.168.1.10")).toBe(true);
  });
});
