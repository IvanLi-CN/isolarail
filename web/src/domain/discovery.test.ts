import { describe, expect, test } from "bun:test";

import {
  applyDiscoveredDeviceToManualForm,
  createInitialDiscoverySnapshot,
  isDiscoveredDeviceAdded,
  parseCidr,
  parseDiscoveredDeviceFromApiInfo,
  reduceDiscoverySnapshot,
} from "./discovery";

describe("applyDiscoveredDeviceToManualForm", () => {
  test("fills baseUrl always and suggests name/id when blank", () => {
    const current = { name: "", baseUrl: "", id: "" };
    const next = applyDiscoveredDeviceToManualForm(current, {
      baseUrl: "http://hub.local",
      hostname: "hub",
      device_id: "abcd1234",
    });
    expect(next).toEqual({
      name: "hub",
      baseUrl: "http://hub.local",
      id: "abcd1234",
    });
  });

  test("does not overwrite name/id when user already typed", () => {
    const current = { name: "My hub", baseUrl: "", id: "custom" };
    const next = applyDiscoveredDeviceToManualForm(current, {
      baseUrl: "http://hub.local",
      hostname: "hub",
      device_id: "abcd1234",
    });
    expect(next).toEqual({
      name: "My hub",
      baseUrl: "http://hub.local",
      id: "custom",
    });
  });
});

describe("isDiscoveredDeviceAdded", () => {
  test("dedupes by device_id or baseUrl", () => {
    expect(
      isDiscoveredDeviceAdded(
        { baseUrl: "http://a.local", device_id: "dev1" },
        ["dev1"],
        [],
      ),
    ).toBe(true);
    expect(
      isDiscoveredDeviceAdded(
        { baseUrl: "http://a.local", device_id: "dev2" },
        [],
        ["http://a.local"],
      ),
    ).toBe(true);
    expect(
      isDiscoveredDeviceAdded(
        { baseUrl: "http://a.local", device_id: "dev2" },
        ["dev1"],
        ["http://b.local"],
      ),
    ).toBe(false);
  });
});

describe("parseDiscoveredDeviceFromApiInfo", () => {
  test("rejects non-product firmware", () => {
    const res = parseDiscoveredDeviceFromApiInfo(
      "http://192.168.1.42",
      {
        device: {
          device_id: "aabbccdd",
          hostname: "isohub-aabbccdd",
          fqdn: "isohub-aabbccdd.local",
          variant: "v3",
          firmware: { name: "other", version: "0.0.0" },
          wifi: { ipv4: "192.168.1.42", is_static: false, state: "connected" },
        },
      },
      "192.168.1.42",
      "2026-01-14T00:00:00.000Z",
    );
    expect(res).toBeNull();
  });

  test("prefers fqdn baseUrl when available", () => {
    const res = parseDiscoveredDeviceFromApiInfo(
      "http://192.168.1.42",
      {
        device: {
          device_id: "aabbccdd",
          hostname: "isohub-aabbccdd",
          fqdn: "isohub-aabbccdd.local",
          variant: "v3",
          firmware: { name: "iso-usb-hub", version: "0.1.0" },
          wifi: { ipv4: "192.168.1.42", is_static: false, state: "connected" },
        },
      },
      "192.168.1.42",
      "2026-01-14T00:00:00.000Z",
    );
    expect(res).toEqual({
      baseUrl: "http://isohub-aabbccdd.local",
      device_id: "aabbccdd",
      hostname: "isohub-aabbccdd",
      fqdn: "isohub-aabbccdd.local",
      ipv4: "192.168.1.42",
      variant: "v3",
      firmware: { name: "iso-usb-hub", version: "0.1.0" },
      last_seen_at: "2026-01-14T00:00:00.000Z",
    });
  });
});

describe("parseCidr", () => {
  test("enumerates hosts and skips network/broadcast for /24", () => {
    const res = parseCidr("192.168.1.0/24", 1024);
    expect(res.ok).toBe(true);
    if (!res.ok) {
      throw new Error("expected ok");
    }
    expect(res.hosts[0]).toBe("192.168.1.1");
    expect(res.hosts.at(-1)).toBe("192.168.1.254");
    expect(res.hosts).toHaveLength(254);
  });

  test("rejects huge ranges by default", () => {
    const res = parseCidr("10.0.0.0/8", 1024);
    expect(res.ok).toBe(false);
    if (res.ok) {
      throw new Error("expected error");
    }
    expect(res.error).toMatch(/too large/i);
  });
});

describe("reduceDiscoverySnapshot", () => {
  test("supports user toggle and scan progress", () => {
    let snap = createInitialDiscoverySnapshot({ status: "unavailable" });
    snap = reduceDiscoverySnapshot(snap, {
      type: "toggle_ip_scan",
      expanded: true,
      expandedBy: "user",
    });
    expect(snap.ipScan?.expanded).toBe(true);
    expect(snap.ipScan?.expandedBy).toBe("user");

    snap = reduceDiscoverySnapshot(snap, {
      type: "start_scan",
      cidr: "192.168.1.0/24",
      total: 254,
    });
    expect(snap.status).toBe("scanning");
    expect(snap.mode).toBe("scan");
    expect(snap.scan?.done).toBe(0);

    snap = reduceDiscoverySnapshot(snap, { type: "scan_progress", done: 10 });
    expect(snap.scan?.done).toBe(10);

    snap = reduceDiscoverySnapshot(snap, { type: "scan_done" });
    expect(snap.status).toBe("ready");
  });
});
