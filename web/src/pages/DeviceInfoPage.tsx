import { Link, useNavigate, useParams } from "react-router";
import { useDeviceRuntime } from "../app/device-runtime";
import { useDevices } from "../app/devices-store";
import { DevicePageTabs } from "../ui/nav/DevicePageTabs";
import { DeviceInfoPanel } from "../ui/panels/DeviceInfoPanel";

export function DeviceInfoPage({
  mode = "hardware",
}: {
  mode?: "hardware" | "info";
}) {
  const { deviceId } = useParams();
  const { getDevice, removeDevice } = useDevices();
  const navigate = useNavigate();
  const runtime = useDeviceRuntime();

  if (!deviceId) {
    return null;
  }

  const device = getDevice(deviceId);
  if (!device) {
    return (
      <div className="flex flex-col gap-3" data-testid="device-not-found">
        <div className="text-lg font-semibold">Device not found</div>
        <div className="text-sm opacity-80">
          Choose an existing device or add a new one.
        </div>
        <div>
          <Link className="link" to="/">
            Back to dashboard
          </Link>
        </div>
      </div>
    );
  }

  const shortId = device.id.length > 6 ? device.id.slice(0, 6) : device.id;
  const title = mode === "hardware" ? "Hardware" : "Info";
  const pageTestId =
    mode === "hardware" ? "device-hardware-page" : "device-info-page";

  return (
    <div className="flex flex-col" data-testid={pageTestId}>
      <div>
        <div className="text-[24px] font-bold">{device.name}</div>
        <div className="mt-2 truncate font-mono text-[12px] font-semibold text-[var(--muted)]">
          {title.toLowerCase()} · id: {shortId} • {device.baseUrl}
        </div>
      </div>

      <div className="mt-4">
        <DevicePageTabs deviceId={deviceId} />
      </div>

      <div className="mt-[22px]">
        <DeviceInfoPanel
          mode={mode}
          device={device}
          transport={runtime.transport(device.id)}
          wifiManagementTransport={runtime.wifiManagementTransport(device.id)}
          loadInfo={() => runtime.deviceInfo(device.id)}
          loadWifiConfig={() => runtime.wifiConfig(device.id)}
          saveWifiConfig={(input) => runtime.saveWifiConfig(device.id, input)}
          clearWifiConfig={() => runtime.clearWifiConfig(device.id)}
          rebootDevice={() => runtime.rebootDevice(device.id)}
          deleteDevice={async () => {
            await removeDevice(device.id);
            navigate("/");
          }}
        />
      </div>
    </div>
  );
}
