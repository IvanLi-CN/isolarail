import type { Meta, StoryObj } from "@storybook/react";

import { DeviceRuntimeProvider } from "../../app/device-runtime";
import { DevicesProvider } from "../../app/devices-store";
import type { StoredDevice } from "../../domain/devices";
import {
  jsonResponse,
  mockFetchDecorator,
} from "../../stories/storybook/mockFetch";
import { ToastProvider } from "../toast/ToastProvider";
import { DeviceDashboardPanel } from "./DeviceDashboardPanel";

const demoDevice: StoredDevice = {
  id: "isohub-a",
  name: "Desk Hub A",
  baseUrl: "http://isohub-a.local",
};

const legacyDevice: StoredDevice = {
  id: "isohub-legacy",
  name: "Legacy Hub",
  baseUrl: "http://isohub-legacy.local",
};

const mockDeviceApi = async (
  input: Parameters<typeof fetch>[0],
  init: Parameters<typeof fetch>[1],
  original: typeof fetch,
) => {
  const url = new URL(
    typeof input === "string"
      ? input
      : input instanceof Request
        ? input.url
        : input.toString(),
  );

  if (
    url.hostname !== "isohub-a.local" &&
    url.hostname !== "isohub-legacy.local"
  ) {
    return original(input, init);
  }

  if (url.pathname === "/api/v1/ports") {
    const hub =
      url.hostname === "isohub-legacy.local"
        ? { upstream_connected: true }
        : {
            upstream_connected: true,
            isolated_usb_fault: false,
            isolated_downstream_connected: true,
            isolated_usb_ready: true,
          };

    return jsonResponse({
      hub,
      ports: [
        {
          portId: "port1",
          label: "Port 1",
          telemetry: {
            status: "ok",
            voltage_mv: 5000,
            current_ma: 420,
            power_mw: 2100,
            sample_uptime_ms: 123_456,
          },
          state: {
            power_enabled: true,
            data_connected: true,
            replugging: false,
            busy: false,
          },
          capabilities: { data_replug: true, power_set: true },
        },
        {
          portId: "port2",
          label: "Port 2",
          telemetry: {
            status: "ok",
            voltage_mv: 9000,
            current_ma: 310,
            power_mw: 2790,
            sample_uptime_ms: 123_456,
          },
          state: {
            power_enabled: false,
            data_connected: false,
            replugging: false,
            busy: false,
          },
          capabilities: { data_replug: true, power_set: true },
        },
        {
          portId: "port3",
          label: "Port 3",
          telemetry: {
            status: "off",
            voltage_mv: 0,
            current_ma: 0,
            power_mw: 0,
            sample_uptime_ms: 123_456,
          },
          state: {
            power_enabled: false,
            data_connected: false,
            replugging: false,
            busy: false,
          },
          capabilities: { data_replug: true, power_set: true },
        },
        {
          portId: "port4",
          label: "Port 4",
          telemetry: {
            status: "error",
            voltage_mv: null,
            current_ma: null,
            power_mw: null,
            sample_uptime_ms: 123_456,
          },
          state: {
            power_enabled: false,
            data_connected: false,
            replugging: true,
            busy: true,
          },
          capabilities: { data_replug: true, power_set: true },
        },
      ],
    });
  }

  if (url.pathname.endsWith("/actions/replug")) {
    return jsonResponse({ accepted: true });
  }

  if (url.pathname.includes("/power")) {
    const enabled = url.searchParams.get("enabled") === "1";
    return jsonResponse({ accepted: true, power_enabled: enabled });
  }

  return original(input, init);
};

const meta: Meta<typeof DeviceDashboardPanel> = {
  title: "Panels/DeviceDashboardPanel",
  component: DeviceDashboardPanel,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
  },
  decorators: [
    mockFetchDecorator(mockDeviceApi),
    (Story, context) => (
      <ToastProvider>
        <DevicesProvider initialDevices={[context.args.device ?? demoDevice]}>
          <DeviceRuntimeProvider>
            <div className="max-w-[980px]">
              <Story />
            </div>
          </DeviceRuntimeProvider>
        </DevicesProvider>
      </ToastProvider>
    ),
  ],
  args: {
    device: demoDevice,
  },
};

export default meta;

type Story = StoryObj<typeof DeviceDashboardPanel>;

export const Default: Story = {};

export const LegacyFirmwareUnknownIsolation: Story = {
  args: {
    device: legacyDevice,
  },
};

export const MobileIsolationBadges: Story = {
  parameters: {
    viewport: {
      defaultViewport: "isohubNarrow",
    },
  },
};

export const HeaderBadgeWrapRegression: Story = {
  parameters: {
    layout: "centered",
  },
  decorators: [
    mockFetchDecorator(mockDeviceApi),
    (Story, context) => (
      <ToastProvider>
        <DevicesProvider initialDevices={[context.args.device ?? demoDevice]}>
          <DeviceRuntimeProvider>
            <div className="w-[760px] max-w-full">
              <Story />
            </div>
          </DeviceRuntimeProvider>
        </DevicesProvider>
      </ToastProvider>
    ),
  ],
};
