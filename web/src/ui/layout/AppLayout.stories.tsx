import type { Meta, StoryObj } from "@storybook/react";
import { MemoryRouter } from "react-router";

import { AddDeviceUiProvider } from "../../app/add-device-ui";
import { CompanionBridgeProvider } from "../../app/companion-bridge-ui";
import { DeviceRuntimeProvider } from "../../app/device-runtime";
import { DevicesProvider } from "../../app/devices-store";
import { ThemeProvider } from "../../app/theme-ui";
import type { StoredDevice } from "../../domain/devices";
import { DeviceListPanel } from "../panels/DeviceListPanel";
import { ToastProvider } from "../toast/ToastProvider";
import { AppLayout } from "./AppLayout";

const devices: StoredDevice[] = [
  { id: "demo-a", name: "Demo Hub A", baseUrl: "http://192.168.1.23" },
  { id: "demo-b", name: "Demo Hub B", baseUrl: "http://usb-hub.local" },
];

const meta: Meta<typeof AppLayout> = {
  title: "Layouts/AppLayout",
  component: AppLayout,
  parameters: {
    layout: "fullscreen",
  },
  decorators: [
    (Story) => (
      <MemoryRouter>
        <CompanionBridgeProvider>
          <ThemeProvider>
            <ToastProvider>
              <DevicesProvider initialDevices={devices}>
                <DeviceRuntimeProvider>
                  <AddDeviceUiProvider
                    existingDeviceIds={devices.map((d) => d.id)}
                    existingDeviceBaseUrls={devices.map((d) => d.baseUrl)}
                    onCreate={async () => ({
                      ok: true,
                      device: devices[0],
                    })}
                  >
                    <div className="h-screen" data-theme="isohub">
                      <Story />
                    </div>
                  </AddDeviceUiProvider>
                </DeviceRuntimeProvider>
              </DevicesProvider>
            </ToastProvider>
          </ThemeProvider>
        </CompanionBridgeProvider>
      </MemoryRouter>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof AppLayout>;

export const Default: Story = {
  args: {
    sidebar: (
      <DeviceListPanel
        devices={devices}
        selectedDeviceId="demo-a"
        onSelect={() => {}}
      />
    ),
    children: (
      <div className="flex flex-col gap-3">
        <div className="text-[24px] font-bold">AppLayout</div>
        <div className="text-[14px] font-medium text-[var(--muted)]">
          This is the top-level layout used by the dashboard pages.
        </div>
      </div>
    ),
  },
};

export const Desktop: Story = {
  ...Default,
  parameters: {
    viewport: { defaultViewport: "isohubDesktop" },
  },
};

export const Mobile: Story = {
  ...Default,
  parameters: {
    viewport: { defaultViewport: "isohubMobile" },
  },
};
