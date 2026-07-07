import type { Meta, StoryObj } from "@storybook/react";
import { MemoryRouter, Route, Routes } from "react-router";

import { AddDeviceUiProvider } from "../app/add-device-ui";
import { CompanionBridgeProvider } from "../app/companion-bridge-ui";
import { DeviceRuntimeProvider } from "../app/device-runtime";
import { DevicesProvider } from "../app/devices-store";
import { ThemeProvider } from "../app/theme-ui";
import type { StoredDevice } from "../domain/devices";
import { ToastProvider } from "../ui/toast/ToastProvider";
import { HardwareDebugPage } from "./HardwareDebugPage";

const devices: StoredDevice[] = [
  {
    id: "f1fb44",
    name: "isolarail-f1fb44",
    baseUrl: "http://isolarail-f1fb44.local",
  },
];

const meta: Meta<typeof HardwareDebugPage> = {
  title: "Pages/HardwareDebugPage",
  component: HardwareDebugPage,
  parameters: {
    layout: "fullscreen",
  },
  decorators: [
    () => (
      <MemoryRouter initialEntries={["/devices/f1fb44/debug/hardware?devd="]}>
        <CompanionBridgeProvider>
          <ThemeProvider>
            <ToastProvider>
              <DevicesProvider initialDevices={devices}>
                <DeviceRuntimeProvider>
                  <AddDeviceUiProvider
                    existingDeviceBaseUrls={devices.map(
                      (device) => device.baseUrl,
                    )}
                    existingDeviceIds={devices.map((device) => device.id)}
                    onCreate={async () => ({
                      ok: true,
                      device: devices[0],
                    })}
                  >
                    <div
                      className="min-h-screen bg-[var(--bg)] p-8"
                      data-theme="isolarail"
                    >
                      <Routes>
                        <Route
                          element={<HardwareDebugPage />}
                          path="/devices/:deviceId/debug/hardware"
                        />
                      </Routes>
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
type Story = StoryObj<typeof HardwareDebugPage>;

export const JsonExplorer: Story = {};

export const Mobile: Story = {
  parameters: {
    viewport: { defaultViewport: "isolarailMobile" },
  },
};
