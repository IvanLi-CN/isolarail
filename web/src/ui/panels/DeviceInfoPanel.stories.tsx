import type { Meta, StoryObj } from "@storybook/react";
import { expect, userEvent, within } from "@storybook/test";

import type {
  DeviceInfoResponse,
  Result,
  WifiConfigResponse,
  WifiMutationResponse,
} from "../../domain/deviceApi";
import type { StoredDevice } from "../../domain/devices";
import type { SerialActivityEntry } from "../../domain/hardwareConsole";
import { DeviceInfoPanel } from "./DeviceInfoPanel";

const demoDevice: StoredDevice = {
  id: "isohub-a",
  name: "Bench Hub A",
  baseUrl: "http://isohub-a.local",
  transports: {
    httpBaseUrl: "http://isohub-a.local",
    localUsbDeviceId: "usb--dev-cu-usbmodem21221401",
    webSerialLabel: "ESP32-S3 USB Serial/JTAG",
  },
};

const mockInfo: DeviceInfoResponse = {
  device: {
    device_id: "isohub-a1b2c3",
    hostname: "isohub-a1b2c3",
    fqdn: "isohub-a1b2c3.local",
    mac: "AA:BB:CC:DD:EE:FF",
    variant: "v3",
    firmware: { name: "iso-usb-hub", version: "0.1.0" },
    uptime_ms: 123_456,
    wifi: { state: "connected", ipv4: "192.168.1.42", is_static: false },
  },
};

const configuredWifi: WifiConfigResponse = {
  configured: true,
  storage: "eeprom",
  address: "0x50",
  ssid: "Bench WiFi",
  psk_configured: true,
  state: "connected",
  ipv4: "192.168.1.42",
  is_static: false,
};

const okInfo = (): Promise<Result<DeviceInfoResponse>> =>
  Promise.resolve({ ok: true, value: mockInfo });

const okWifi = (
  value: WifiConfigResponse = configuredWifi,
): Promise<Result<WifiConfigResponse>> => Promise.resolve({ ok: true, value });

const okWifiMutation = (
  rebootRequired = false,
): Promise<Result<WifiMutationResponse>> =>
  Promise.resolve({
    ok: true,
    value: { accepted: true, reboot_required: rebootRequired },
  });

const neverInfo = (): Promise<Result<DeviceInfoResponse>> =>
  new Promise(() => undefined);

const neverWifi = (): Promise<Result<WifiConfigResponse>> =>
  new Promise(() => undefined);

const offlineInfo = (): Promise<Result<DeviceInfoResponse>> =>
  Promise.resolve({
    ok: false,
    error: {
      kind: "offline",
      message: "Waiting for an active connection.",
    },
  });

const offlineWifi = (): Promise<Result<WifiConfigResponse>> =>
  Promise.resolve({
    ok: false,
    error: {
      kind: "offline",
      message: "Waiting for an active connection.",
    },
  });

const serialPreview: SerialActivityEntry[] = [
  {
    id: "activity-3",
    channel: "web_serial",
    kind: "json",
    summary: "json response 42",
    payload: '{"id":"42","ok":true,"result":{"accepted":true}}',
    requestId: "42",
    timestampMs: Date.now(),
  },
  {
    id: "activity-2",
    channel: "web_serial",
    kind: "raw",
    summary: "raw cdc line",
    payload: "boot: wifi provisioning pending",
    requestId: null,
    timestampMs: Date.now() - 400,
  },
  {
    id: "activity-1",
    channel: "web_serial",
    kind: "defmt",
    summary: "defmt/raw binary frame",
    payload: "ff 00 91 92 93 00",
    requestId: null,
    timestampMs: Date.now() - 800,
  },
];

const meta: Meta<typeof DeviceInfoPanel> = {
  title: "Panels/DeviceInfoPanel",
  component: DeviceInfoPanel,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  decorators: [
    (Story) => (
      <div className="min-h-screen bg-[var(--bg)] p-6">
        <div className="mx-auto max-w-[1180px]">
          <Story />
        </div>
      </div>
    ),
  ],
  args: {
    mode: "hardware",
    device: demoDevice,
    connectionState: "online",
    lastOkAt: Date.now(),
    lastErrorLabel: null,
    transport: "local_usb",
    channelStates: {
      http: "offline",
      web_serial: "unknown",
      local_usb: "online",
    },
    wifiManagementTransport: "local_usb",
    loadInfo: okInfo,
    loadWifiConfig: () => okWifi(),
    saveWifiConfig: () => okWifiMutation(false),
    clearWifiConfig: () => okWifiMutation(false),
    rebootDevice: async () => ({ ok: true, value: { accepted: true } }),
    deleteDevice: async () => undefined,
  },
};

export default meta;

type Story = StoryObj<typeof DeviceInfoPanel>;

export const SettingsMaintenance: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Wi-Fi configuration")).toBeVisible();
    await expect(canvas.getByText("Firmware update")).toBeVisible();
    await expect(canvas.getByText("Serial activity")).toBeVisible();
    await expect(canvas.getByText("Danger actions")).toBeVisible();
    await expect(canvas.queryByText("Identity")).not.toBeInTheDocument();
    await expect(
      canvas.queryByText("Connection channels"),
    ).not.toBeInTheDocument();
  },
};

export const InfoSummary: Story = {
  args: {
    mode: "info",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Identity")).toBeVisible();
    await expect(canvas.getByText("Firmware")).toBeVisible();
    await expect(canvas.getByText("Connection channels")).toBeVisible();
    await expect(canvas.getByText("Saved profile metadata")).toBeVisible();
    await expect(canvas.getByText("Last seen")).toBeVisible();
    await expect(
      canvas.queryByText("Wi-Fi configuration"),
    ).not.toBeInTheDocument();
    await expect(canvas.queryByText("Firmware update")).not.toBeInTheDocument();
    await expect(canvas.queryByText("Danger actions")).not.toBeInTheDocument();
  },
};

export const WebSerialActivity: Story = {
  args: {
    transport: "web_serial",
    wifiManagementTransport: "web_serial",
    serialActivityPreview: serialPreview,
  },
};

export const LoadingHardwareTelemetry: Story = {
  args: {
    mode: "info",
    loadInfo: neverInfo,
    loadWifiConfig: neverWifi,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Identity")).toBeVisible();
    await expect(canvas.queryByText("unknown")).not.toBeInTheDocument();
    await expect(
      canvasElement.querySelector(".iso-skeleton-line"),
    ).not.toBeNull();
  },
};

export const WaitingForConnection: Story = {
  args: {
    mode: "info",
    transport: null,
    wifiManagementTransport: null,
    loadInfo: offlineInfo,
    loadWifiConfig: offlineWifi,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Identity")).toBeVisible();
    await expect(
      await canvas.findByText(
        "Not connected info unavailable: Waiting for an active connection.",
      ),
    ).toBeVisible();
    await expect(canvas.getAllByText("—").length).toBeGreaterThan(0);
    await expect(canvas.queryByText("unknown")).not.toBeInTheDocument();
  },
};

export const LanReadOnly: Story = {
  args: {
    mode: "hardware",
    transport: "http",
    wifiManagementTransport: null,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByText("Current: Wi-Fi / LAN")[0]).toBeVisible();
    await expect(
      canvas.getByRole("button", { name: "Save Wi-Fi" }),
    ).toBeDisabled();
  },
};

export const InvalidShortPsk: Story = {
  args: {
    transport: "local_usb",
    wifiManagementTransport: "local_usb",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const pskInput = await canvas.findByLabelText("PSK");
    await userEvent.clear(pskInput);
    await userEvent.type(pskInput, "short");
    await userEvent.click(canvas.getByRole("button", { name: "Save Wi-Fi" }));
    await expect(
      await canvas.findByText(/PSK must be blank or at least 8 bytes/),
    ).toBeVisible();
  },
};

export const NarrowWebSerial: Story = {
  parameters: {
    viewport: { defaultViewport: "isohubNarrow" },
  },
  args: {
    transport: "web_serial",
    wifiManagementTransport: "web_serial",
    serialActivityPreview: serialPreview,
  },
};
