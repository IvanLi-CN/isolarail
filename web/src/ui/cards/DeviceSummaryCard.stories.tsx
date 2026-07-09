import type { Meta, StoryObj } from "@storybook/react";
import type { StoredDevice } from "../../domain/devices";
import type { PortState, PortTelemetry } from "../../domain/ports";
import { DeviceSummaryCard } from "./DeviceSummaryCard";

const demoDevice: StoredDevice = {
  id: "isolarail-a",
  name: "Desk Hub A",
  baseUrl: "http://isolarail-a.local",
};

const okTelemetry: PortTelemetry = {
  status: "ok",
  voltage_mv: 5080,
  current_ma: 420,
  power_mw: 2130,
  sample_uptime_ms: 120_000,
};

const offTelemetry: PortTelemetry = {
  status: "off",
  voltage_mv: 0,
  current_ma: 0,
  power_mw: 0,
  sample_uptime_ms: 120_000,
};

const errorTelemetry: PortTelemetry = {
  status: "error",
  voltage_mv: null,
  current_ma: null,
  power_mw: null,
  sample_uptime_ms: 120_000,
};

const idleState: PortState = {
  power_enabled: false,
  data_connected: false,
  replugging: false,
  busy: false,
  overcurrent: false,
};

const liveState: PortState = {
  power_enabled: true,
  data_connected: true,
  replugging: false,
  busy: false,
  overcurrent: false,
};

const sampleLastOkAt = 1_700_000_000_000;

const meta: Meta<typeof DeviceSummaryCard> = {
  title: "Cards/DeviceSummaryCard",
  component: DeviceSummaryCard,
  decorators: [
    (Story) => (
      <div className="max-w-[720px]">
        <Story />
      </div>
    ),
  ],
  args: {
    device: demoDevice,
    connection: { state: "online", lastOkAt: sampleLastOkAt },
    upstreamConnected: true,
    ports: {
      port1: { label: "Port 1", telemetry: okTelemetry, state: liveState },
      port2: { label: "Port 2", telemetry: okTelemetry, state: liveState },
      port3: { label: "Port 3", telemetry: offTelemetry, state: idleState },
      port4: { label: "Port 4", telemetry: errorTelemetry, state: idleState },
    },
    onOpenDashboard: () => {},
    onSetPower: () => {},
    onDataReplug: () => {},
  },
};

export default meta;

type Story = StoryObj<typeof DeviceSummaryCard>;

export const Default: Story = {};

export const Offline: Story = {
  args: {
    connection: { state: "offline", lastOkAt: sampleLastOkAt },
    upstreamConnected: null,
  },
};
