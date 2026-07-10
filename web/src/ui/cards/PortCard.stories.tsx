import type { Meta, StoryObj } from "@storybook/react";
import { expect, within } from "@storybook/test";

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

PowerOn.play = async ({ canvasElement }) => {
  const canvas = within(canvasElement);
  await expect(canvas.getByText("ON")).toBeVisible();
  await expect(canvas.getByText("Cut")).toBeVisible();
  await expect(canvasElement.querySelectorAll("svg").length).toBeGreaterThan(0);
  await expect(canvas.queryByText("State")).not.toBeInTheDocument();
  await expect(canvas.queryByText("Data linked")).not.toBeInTheDocument();
  await expect(canvas.queryByText("Data off")).not.toBeInTheDocument();
};

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

PowerOff.play = async ({ canvasElement }) => {
  const canvas = within(canvasElement);
  await expect(canvas.getByText("OFF")).toBeVisible();
  await expect(canvas.getByText("Restore")).toBeVisible();
  await expect(canvasElement.querySelectorAll("svg").length).toBeGreaterThan(0);
  await expect(canvas.queryByText("State")).not.toBeInTheDocument();
  await expect(canvas.queryByText("Data linked")).not.toBeInTheDocument();
  await expect(canvas.queryByText("Data off")).not.toBeInTheDocument();
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
    powerPending: true,
    state: {
      power_enabled: true,
      data_connected: true,
      replugging: false,
      busy: true,
    },
  },
};

Busy.play = async ({ canvasElement }) => {
  const powerIcon = canvasElement.querySelector(".lucide-power");
  const loaderIcon = canvasElement.querySelector(".lucide-loader-circle");
  await expect(powerIcon).toBeNull();
  await expect(loaderIcon).toBeTruthy();
  await expect(loaderIcon).toHaveClass("iso-control-spin");
};

export const BusyNotPending: Story = {
  args: {
    portId: "port2",
    label: "Port 2",
    powerPending: false,
    state: {
      power_enabled: false,
      data_connected: false,
      replugging: false,
      busy: true,
    },
    telemetry: {
      status: "error",
      voltage_mv: null,
      current_ma: null,
      power_mw: null,
      sample_uptime_ms: 0,
    },
    disabled: true,
  },
};

BusyNotPending.play = async ({ canvasElement }) => {
  const powerIcon = canvasElement.querySelector(".lucide-power");
  const loaderIcon = canvasElement.querySelector(".lucide-loader-circle");
  await expect(powerIcon).toBeTruthy();
  await expect(loaderIcon).toBeNull();
};

export const ConfirmingPowerOff: Story = {};

ConfirmingPowerOff.play = async ({ canvasElement, userEvent }) => {
  const canvas = within(canvasElement);
  await userEvent.click(
    canvas.getByRole("button", { name: /power on, turn off/i }),
  );
  await expect(canvas.getByText("Cut power to Port 1?")).toBeVisible();
  await expect(canvas.getByRole("button", { name: "Cancel" })).toBeVisible();
  await expect(canvas.getByRole("button", { name: "Cut power" })).toBeVisible();
};
