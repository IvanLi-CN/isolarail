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
                    <div className="h-screen" data-theme="isolarail">
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
      <div className="flex flex-col gap-5">
        <div className="iso-panel px-5 py-5 sm:px-6">
          <div className="iso-kicker">story surface</div>
          <div className="mt-2 text-[30px] font-black leading-[0.94] tracking-[-0.03em]">
            App layout shell
          </div>
          <div className="mt-3 text-[14px] font-medium leading-[1.6] text-[var(--muted)]">
            Preview the operator shell, sidebar framing, and console pacing used
            by dashboard pages.
          </div>
        </div>
        <div className="iso-panel-subtle px-4 py-3 text-[12px] font-semibold leading-[1.55] text-[var(--muted)]">
          This story exercises the top-level frame only; device detail surfaces
          live in their own cards and panel stories.
        </div>
      </div>
    ),
  },
};

export const Desktop: Story = {
  ...Default,
  parameters: {
    viewport: { defaultViewport: "isolarailDesktop" },
  },
};

export const Mobile: Story = {
  ...Default,
  parameters: {
    viewport: { defaultViewport: "isolarailMobile" },
  },
};
