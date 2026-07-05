import { expect, test } from "bun:test";
import { deviceIdFromPath, mockSnapshot, summarize } from "./hardwareSnapshot";

test("extracts the device id from the advanced hardware route", () => {
  expect(deviceIdFromPath("/devices/local/debug/hardware")).toBe("local");
  expect(deviceIdFromPath("/devices/hub%201/debug/hardware")).toBe("hub 1");
});

test("summarizes mixed online and offline modules", () => {
  const summary = summarize(mockSnapshot);
  expect(summary.onlinePorts).toBe(2);
  expect(summary.degradedPorts).toBe(2);
  expect(summary.frontPanel).toBe("offline");
  expect(summary.sideband).toBe("online");
});
