import { Link, useParams } from "react-router";
import { useDevices } from "../app/devices-store";
import { DevicePageTabs } from "../ui/nav/DevicePageTabs";
import { DeviceDashboardPanel } from "../ui/panels/DeviceDashboardPanel";

export function DeviceDashboardPage() {
  const { deviceId } = useParams();
  const { getDevice } = useDevices();

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

  return (
    <div className="flex flex-col gap-5" data-testid="device-dashboard-page">
      <div className="iso-panel px-5 py-5 sm:px-6">
        <div className="iso-kicker">claimed route</div>
        <div className="mt-2 text-[30px] font-black leading-[0.94] tracking-[-0.05em]">
          {device.name}
        </div>
        <div className="mt-3 truncate font-mono text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--muted)]">
          dashboard · id: {shortId} · {device.baseUrl}
        </div>
        <div className="mt-4 flex flex-wrap gap-2">
          <span className="iso-chip iso-chip--signal">device dashboard</span>
          <span className="iso-chip">per-port rail control</span>
          <span className="iso-chip iso-chip--trace">measured telemetry</span>
        </div>
      </div>

      <DevicePageTabs deviceId={deviceId} />

      <DeviceDashboardPanel device={device} />
    </div>
  );
}
