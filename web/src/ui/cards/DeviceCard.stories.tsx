import type { Meta, StoryObj } from "@storybook/react";

import type { StoredDevice } from "../../domain/devices";
import { DeviceCard } from "./DeviceCard";

const demoDevice: StoredDevice = {
  id: "isolarail-a",
  name: "Desk Hub A",
  baseUrl: "http://isolarail-a.local",
};

const meta: Meta<typeof DeviceCard> = {
  title: "Cards/DeviceCard",
  component: DeviceCard,
  args: {
    device: demoDevice,
    status: "online",
    transportBadges: [{ transport: "http", state: "primary" }],
    unselectedFill: "panel",
    onSelect: () => {},
  },
};

export default meta;

type Story = StoryObj<typeof DeviceCard>;

export const Default: Story = {};

export const ConnectedAndHistory: Story = {
  args: {
    transportBadges: [
      { transport: "web_serial", state: "primary" },
      { transport: "http", state: "connected" },
      { transport: "local_usb", state: "history" },
    ],
  },
};
