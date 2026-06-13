import { describe, expect, test } from "bun:test";

import { normalizeBaseUrl, validateAddDeviceInput } from "./devices";

describe("normalizeBaseUrl", () => {
  test("requires a valid http/https url", () => {
    expect(normalizeBaseUrl("")).toEqual({
      ok: false,
      error: "Base URL is required",
    });
    expect(normalizeBaseUrl("not a url")).toEqual({
      ok: false,
      error: "Base URL must be a valid URL",
    });
    expect(normalizeBaseUrl("ftp://example.com")).toEqual({
      ok: false,
      error: "Base URL must start with http:// or https://",
    });
  });

  test("normalizes to origin", () => {
    const result = normalizeBaseUrl("http://example.com/foo/bar");
    expect(result.ok).toBe(true);
    if (!result.ok) {
      throw new Error("expected ok");
    }
    expect(result.baseUrl).toBe("http://example.com");
  });
});

describe("validateAddDeviceInput", () => {
  test("requires name and baseUrl", () => {
    const res = validateAddDeviceInput({ name: "", baseUrl: "" });
    expect(res.ok).toBe(false);
    if (res.ok) {
      throw new Error("expected errors");
    }
    expect(res.errors.name).toBeDefined();
    expect(res.errors.baseUrl).toBeDefined();
  });

  test("rejects duplicate id", () => {
    const res = validateAddDeviceInput(
      { name: "A", baseUrl: "http://example.com", id: "dup" },
      ["dup"],
    );
    expect(res.ok).toBe(false);
    if (res.ok) {
      throw new Error("expected errors");
    }
    expect(res.errors.id).toBe("ID already exists");
  });

  test("rejects duplicate baseUrl", () => {
    const res = validateAddDeviceInput(
      { name: "A", baseUrl: "http://example.com" },
      [],
      ["http://example.com"],
    );
    expect(res.ok).toBe(false);
    if (res.ok) {
      throw new Error("expected errors");
    }
    expect(res.errors.baseUrl).toBe("Base URL already exists");
  });

  test("generates id when missing", () => {
    const res = validateAddDeviceInput({
      name: "A",
      baseUrl: "http://example.com",
    });
    expect(res.ok).toBe(true);
    if (!res.ok) {
      throw new Error("expected ok");
    }
    expect(res.device.id).toHaveLength(8);
  });
});
