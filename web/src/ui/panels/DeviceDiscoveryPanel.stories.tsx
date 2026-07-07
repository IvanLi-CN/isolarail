import type { Meta, StoryObj } from "@storybook/react";

import type { DiscoverySnapshot } from "../../domain/discovery";
import { DeviceDiscoveryPanel } from "./DeviceDiscoveryPanel";

const baseSnapshot: DiscoverySnapshot = {
  mode: "service",
  status: "unavailable",
  devices: [],
  ipScan: { expanded: false, autoExpandAfterMs: 30_000 },
};

const meta: Meta<typeof DeviceDiscoveryPanel> = {
  title: "Panels/DeviceDiscoveryPanel",
  component: DeviceDiscoveryPanel,
  parameters: {
    layout: "fullscreen",
  },
  decorators: [
    (Story) => (
      <div className="min-h-screen bg-[var(--bg)] p-8" data-theme="isolarail">
        <div className="h-[680px] w-[480px]">
          <Story />
        </div>
      </div>
    ),
  ],
  args: {
    snapshot: baseSnapshot,
    existingDeviceIds: ["f293cc"],
    existingDeviceBaseUrls: ["http://isolarail-f293cc.local"],
    onRefresh: () => {},
    onToggleIpScan: () => {},
    onStartScan: () => {},
    onCancelScan: () => {},
    onSelect: () => {},
  },
};

export default meta;
type Story = StoryObj<typeof DeviceDiscoveryPanel>;

export const WebUnavailable: Story = {
  args: {
    snapshot: baseSnapshot,
  },
};

export const WithResults: Story = {
  args: {
    snapshot: {
      mode: "scan",
      status: "ready",
      devices: [
        {
          baseUrl: "http://isolarail-f293cc.local",
          device_id: "f293cc",
          hostname: "isolarail-f293cc",
          fqdn: "isolarail-f293cc.local",
          ipv4: "192.168.31.224",
          firmware: { name: "isolarail", version: "0.1.0" },
          last_seen_at: "2026-01-14T00:00:00.000Z",
        },
        {
          baseUrl: "http://isolarail-a1b2c3.local",
          device_id: "a1b2c3",
          hostname: "isolarail-a1b2c3",
          fqdn: "isolarail-a1b2c3.local",
          ipv4: "192.168.31.233",
          firmware: { name: "isolarail", version: "0.1.0" },
          last_seen_at: "2026-01-14T00:00:00.000Z",
        },
      ],
      ipScan: { expanded: true, expandedBy: "user" },
      scan: { cidr: "192.168.31.0/24", done: 254, total: 254 },
    },
  },
};

export const ScanningIpScan: Story = {
  args: {
    snapshot: {
      mode: "scan",
      status: "scanning",
      devices: [],
      ipScan: { expanded: true, expandedBy: "auto" },
      scan: { cidr: "192.168.31.0/24", done: 42, total: 254 },
    },
  },
};
