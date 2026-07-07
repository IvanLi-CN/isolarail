import { describe, expect, test } from "bun:test";
import { DEVICES_STORAGE_KEY } from "../domain/devices";
import { readMigrationPayload } from "./storage-migration";
import { THEME_STORAGE_KEY } from "./theme";

describe("readMigrationPayload", () => {
  test("preserves stored transport metadata during migration", () => {
    const localStorage = new Map<string, string>();
    globalThis.window = {
      localStorage: {
        getItem: (key: string) => localStorage.get(key) ?? null,
      },
    } as typeof window;

    localStorage.set(
      DEVICES_STORAGE_KEY,
      JSON.stringify([
        {
          id: "demo",
          name: "Demo Hub",
          baseUrl: "http://isolarail-demo.local/path",
          lastSeenAt: "2026-01-01T00:00:00.000Z",
          transports: {
            httpBaseUrl: "http://192.168.1.23/status",
            localUsbDeviceId: "usb--dev--cu-usbmodem21234101",
            webSerialLabel: "ESP32-S3",
          },
        },
      ]),
    );
    localStorage.set(THEME_STORAGE_KEY, JSON.stringify("isolarail"));

    expect(readMigrationPayload()).toEqual({
      devices: [
        {
          id: "demo",
          name: "Demo Hub",
          baseUrl: "http://isolarail-demo.local",
          lastSeenAt: "2026-01-01T00:00:00.000Z",
          transports: {
            httpBaseUrl: "http://192.168.1.23",
            localUsbDeviceId: "usb--dev--cu-usbmodem21234101",
            webSerialLabel: "ESP32-S3",
          },
        },
      ],
      settings: { theme: "isolarail" },
    });
  });
});
