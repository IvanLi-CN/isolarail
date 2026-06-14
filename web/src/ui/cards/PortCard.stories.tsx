import type { Meta, StoryObj } from "@storybook/react";

import { PortCard } from "./PortCard";

const meta: Meta<typeof PortCard> = {
  title: "Cards/PortCard",
  component: PortCard,
  tags: ["autodocs"],
  args: {
    label: "Port 1",
    portId: "port1",
    telemetry: {
      status: "ok",
      voltage_mv: 5030,
      current_ma: 820,
      power_mw: Math.round((5030 * 820) / 1000),
      sample_uptime_ms: 123_450,
    },
    state: {
      power_enabled: true,
      data_connected: true,
      replugging: false,
      busy: false,
    },
    onTogglePower: () => {},
    onReplug: () => {},
  },
};

export default meta;
type Story = StoryObj<typeof PortCard>;

export const PowerOn: Story = {};

export const PowerOff: Story = {
  args: {
    state: {
      power_enabled: false,
      data_connected: false,
      replugging: false,
      busy: false,
    },
    telemetry: {
      status: "off",
      voltage_mv: 0,
      current_ma: 0,
      power_mw: 0,
      sample_uptime_ms: 123_999,
    },
  },
};

export const NotInserted: Story = {
  args: {
    state: {
      power_enabled: false,
      data_connected: false,
      replugging: false,
      busy: false,
    },
    telemetry: {
      status: "not_inserted",
      voltage_mv: 0,
      current_ma: 0,
      power_mw: 0,
      sample_uptime_ms: 124_000,
    },
  },
};

export const Replugging: Story = {
  args: {
    state: {
      power_enabled: true,
      data_connected: false,
      replugging: true,
      busy: true,
    },
  },
};

export const Busy: Story = {
  args: {
    portId: "port4",
    label: "Port 4",
    state: {
      power_enabled: true,
      data_connected: true,
      replugging: false,
      busy: true,
    },
  },
};
