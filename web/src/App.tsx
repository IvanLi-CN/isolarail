import {
  BrowserRouter,
  Navigate,
  Outlet,
  Route,
  Routes,
  useNavigate,
  useParams,
} from "react-router";
import { AddDeviceUiProvider } from "./app/add-device-ui";
import { CompanionBridgeProvider } from "./app/companion-bridge-ui";
import { DeviceRuntimeProvider } from "./app/device-runtime";
import { DevicesProvider, useDevices } from "./app/devices-store";
import { ThemeProvider } from "./app/theme-ui";
import type { AddDeviceInput } from "./domain/devices";
import { AboutPage } from "./pages/AboutPage";
import { DashboardPage } from "./pages/DashboardPage";
import { DeviceDashboardPage } from "./pages/DeviceDashboardPage";
import { DeviceInfoPage } from "./pages/DeviceInfoPage";
import { NotFoundPage } from "./pages/NotFoundPage";
import { AppLayout } from "./ui/layout/AppLayout";
import { DeviceListPanel } from "./ui/panels/DeviceListPanel";
import { ToastProvider } from "./ui/toast/ToastProvider";

function RootLayout() {
  const { deviceId } = useParams();
  const { devices, addDevice, upsertDevice } = useDevices();
  const navigate = useNavigate();

  const existingIds = devices.map((d) => d.id);
  const existingBaseUrls = devices.map((d) => d.baseUrl);
  const existingNamesById = Object.fromEntries(
    devices.map((d) => [d.id, d.name]),
  );

  const onAdd = async (input: AddDeviceInput) => {
    const result = await addDevice(input);
    if (!result.ok) {
      return result;
    }
    navigate(`/devices/${result.device.id}`);
    return result;
  };

  return (
    <AddDeviceUiProvider
      existingDeviceIds={existingIds}
      existingDeviceBaseUrls={existingBaseUrls}
      existingDeviceNamesById={existingNamesById}
      onCreate={onAdd}
      onUpsert={upsertDevice}
    >
      <AppLayout
        sidebar={
          <DeviceListPanel
            devices={devices}
            selectedDeviceId={deviceId}
            onSelect={(id) => navigate(`/devices/${id}`)}
          />
        }
      >
        <Outlet />
      </AppLayout>
    </AddDeviceUiProvider>
  );
}

function LegacyDeviceRouteRedirect({
  target,
}: {
  target: "dashboard" | "info";
}) {
  const { deviceId } = useParams();

  if (!deviceId) {
    return <Navigate to="/" replace />;
  }

  return (
    <Navigate
      to={
        target === "dashboard"
          ? `/devices/${deviceId}`
          : `/devices/${deviceId}/info`
      }
      replace
    />
  );
}

export default function App() {
  return (
    <BrowserRouter basename={import.meta.env.BASE_URL}>
      <CompanionBridgeProvider>
        <ThemeProvider>
          <ToastProvider>
            <DevicesProvider>
              <DeviceRuntimeProvider>
                <Routes>
                  <Route path="/" element={<RootLayout />}>
                    <Route index element={<DashboardPage />} />
                    <Route
                      path="devices/:deviceId"
                      element={<DeviceDashboardPage />}
                    />
                    <Route
                      path="devices/:deviceId/hardware"
                      element={<DeviceInfoPage mode="hardware" />}
                    />
                    <Route
                      path="devices/:deviceId/info"
                      element={<DeviceInfoPage mode="info" />}
                    />
                    <Route
                      path="devices/:deviceId/details"
                      element={<LegacyDeviceRouteRedirect target="info" />}
                    />
                    <Route
                      path="devices/:deviceId/overview"
                      element={<LegacyDeviceRouteRedirect target="dashboard" />}
                    />
                    <Route path="about" element={<AboutPage />} />
                    <Route path="*" element={<NotFoundPage />} />
                  </Route>
                </Routes>
              </DeviceRuntimeProvider>
            </DevicesProvider>
          </ToastProvider>
        </ThemeProvider>
      </CompanionBridgeProvider>
    </BrowserRouter>
  );
}
