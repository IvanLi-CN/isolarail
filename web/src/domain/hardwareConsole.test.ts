import { describe, expect, test } from "bun:test";

import {
  devdLocalUsbDeviceIdFromBaseUrl,
  filterEsp32SerialPorts,
  isEsp32SerialPort,
  stableLocalUsbDeviceId,
  WebSerialJsonlTransport,
} from "./hardwareConsole";

describe("isEsp32SerialPort", () => {
  test("accepts ESP32-S3 USB Serial/JTAG by USB metadata across platforms", () => {
    expect(
      isEsp32SerialPort({
        path: "COM3",
        label: "USB JTAG/serial debug unit",
        vendorId: 0x303a,
        productId: 0x1001,
      }),
    ).toBe(true);
    expect(
      isEsp32SerialPort({
        path: "/dev/ttyACM0",
        label: "USB JTAG/serial debug unit",
        vendorId: 0x303a,
        productId: 0x1001,
      }),
    ).toBe(true);
  });

  test("keeps unrelated local ports out of Local USB choices", () => {
    expect(
      isEsp32SerialPort({
        path: "/dev/cu.Bluetooth-Incoming-Port",
        label: "Bluetooth-Incoming-Port",
      }),
    ).toBe(false);
    expect(
      isEsp32SerialPort({
        path: "/dev/cu.debug-console",
        label: "debug console",
      }),
    ).toBe(false);
  });
});

describe("filterEsp32SerialPorts", () => {
  test("dedupes tty/cu pairs after filtering ESP32 candidates", () => {
    const ports = filterEsp32SerialPorts([
      {
        path: "/dev/tty.usbmodem21221401",
        label: "USB JTAG/serial debug unit",
        vendorId: 0x303a,
        productId: 0x1001,
      },
      {
        path: "/dev/cu.usbmodem21221401",
        label: "USB JTAG/serial debug unit",
        vendorId: 0x303a,
        productId: 0x1001,
      },
    ]);

    expect(ports).toHaveLength(1);
    expect(ports[0]?.path).toBe("/dev/cu.usbmodem21221401");
  });
});

describe("stableLocalUsbDeviceId", () => {
  test("matches devd USB device id derivation", () => {
    expect(stableLocalUsbDeviceId("/dev/cu.usbmodem21221401")).toBe(
      "usb--dev-cu-usbmodem21221401",
    );
  });
});

describe("devdLocalUsbDeviceIdFromBaseUrl", () => {
  test("extracts CLI/devd USB profile ids", () => {
    expect(
      devdLocalUsbDeviceIdFromBaseUrl(
        "isohub-devd://usb--dev-cu-usbmodem21221401",
      ),
    ).toBe("usb--dev-cu-usbmodem21221401");
    expect(devdLocalUsbDeviceIdFromBaseUrl("http://192.168.4.1")).toBeNull();
  });
});

describe("WebSerialJsonlTransport", () => {
  test("extracts JSONL responses from mixed CDC bytes", async () => {
    const transport = new WebSerialJsonlTransport();
    const events: string[] = [];
    transport.subscribeActivity((entry) => {
      events.push(`${entry.kind}:${entry.summary}`);
    });
    const response = Promise.withResolvers<unknown>();
    const pending = new Map<
      string,
      {
        resolve: (value: unknown) => void;
        reject: (err: Error) => void;
        timeoutId: number;
      }
    >();
    pending.set("42", {
      resolve: response.resolve,
      reject: response.reject,
      timeoutId: 1,
    });

    Object.assign(transport as object, {
      pending,
      decoder: new TextDecoder(),
      readBuffer: [],
    });

    (
      transport as unknown as {
        consumeMonitorBytes(bytes: ArrayLike<number>): void;
      }
    ).consumeMonitorBytes(
      Uint8Array.from([
        0xff,
        0x00,
        0x91,
        0x92,
        0x93,
        0x00,
        0x78,
        0x79,
        0x7a,
        ...new TextEncoder().encode(
          '{"id":"42","ok":true,"result":{"status":"ok"}}\n',
        ),
      ]),
    );

    await expect(response.promise).resolves.toEqual({
      id: "42",
      ok: true,
      result: {
        status: "ok",
      },
    });
    expect(events.some((entry) => entry.startsWith("defmt:"))).toBe(true);
    expect(
      events.some((entry) => entry.startsWith("json:json response 42")),
    ).toBe(true);
  });

  test("records raw UTF-8 serial lines for the monitor panel", () => {
    const transport = new WebSerialJsonlTransport();
    const events: string[] = [];
    transport.subscribeActivity((entry) => {
      events.push(`${entry.kind}:${entry.payload}`);
    });

    Object.assign(transport as object, {
      decoder: new TextDecoder(),
      readBuffer: [],
      defmtBuffer: [],
      defmtInFrame: false,
    });

    (
      transport as unknown as {
        consumeMonitorBytes(bytes: ArrayLike<number>): void;
      }
    ).consumeMonitorBytes(
      new TextEncoder().encode("boot: wifi provisioning pending\n"),
    );

    expect(events).toContain("raw:boot: wifi provisioning pending");
  });
});
