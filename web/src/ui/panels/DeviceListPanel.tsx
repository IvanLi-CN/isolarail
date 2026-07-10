import { useAddDeviceUi } from "../../app/add-device-ui";
import { useDeviceRuntime } from "../../app/device-runtime";
import type { StoredDevice } from "../../domain/devices";
import { DeviceCard, type DeviceTransportBadge } from "../cards/DeviceCard";

const TRANSPORT_ORDER: DeviceTransportBadge["transport"][] = [
  "http",
  "web_serial",
  "local_usb",
];

export type DeviceListPanelProps = {
  devices: StoredDevice[];
  selectedDeviceId?: string;
  onSelect: (deviceId: string) => void;
};

export function DeviceListPanel({
  devices,
  selectedDeviceId,
  onSelect,
}: DeviceListPanelProps) {
  const { openAddDevice } = useAddDeviceUi();
  const { connectionState, transport, channelState, runtimeById } =
    useDeviceRuntime();

  const transportBadges = (deviceId: string): DeviceTransportBadge[] => {
    const current = transport(deviceId);
    const channels = runtimeById[deviceId]?.channels;
    if (!channels) {
      return [];
    }
    return TRANSPORT_ORDER.flatMap((candidate) => {
      const channel = channels[candidate];
      const hasHistory = Boolean(channel?.lastOkAt || channel?.lastError);
      if (!hasHistory) {
        return [];
      }
      const state =
        candidate === current && channelState(deviceId, candidate) === "online"
          ? "primary"
          : channelState(deviceId, candidate) === "online"
            ? "connected"
            : "history";
      return [{ transport: candidate, state }];
    });
  };

  return (
    <div
      className="flex h-full min-h-0 flex-col px-4 py-4"
      data-testid="device-list"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-[12px] font-semibold tracking-[0.04em] text-[var(--primary)]">
            Claimed benches
          </div>
          <h2 className="mt-2 text-[24px] font-black leading-[0.96] tracking-[-0.025em]">
            Devices
          </h2>
          <div className="mt-2 text-[13px] font-medium leading-[1.55] text-[var(--muted)]">
            Choose a route, then inspect rails and measured power state.
          </div>
        </div>
        <button
          className="iso-button iso-button--primary shrink-0"
          type="button"
          onClick={() => openAddDevice()}
        >
          Add Device
        </button>
      </div>

      <div className="mt-4 flex flex-wrap gap-x-4 gap-y-2 text-[12px] font-semibold text-[var(--muted)]">
        <span>Route list</span>
        <span>USB / serial / Wi-Fi</span>
        <span>{devices.length} saved</span>
      </div>

      {devices.length === 0 ? (
        <div className="iso-panel mt-4 px-4 py-4">
          <div className="text-[12px] font-semibold text-[var(--muted)]">
            Empty bench
          </div>
          <div className="mt-2 text-[14px] font-semibold text-[var(--text)]">
            No devices claimed yet.
          </div>
          <div className="mt-2 text-[13px] font-medium leading-[1.55] text-[var(--muted)]">
            Add a device to start route selection, telemetry, and per-port
            control.
          </div>
        </div>
      ) : (
        <div className="mt-4 min-h-0 flex-1 overflow-y-auto">
          <div className="flex flex-col gap-3 pr-1">
            {devices.map((d) => (
              <DeviceCard
                key={d.id}
                device={d}
                selected={d.id === selectedDeviceId}
                status={connectionState(d.id)}
                transportBadges={transportBadges(d.id)}
                unselectedFill={selectedDeviceId ? "panel-2" : "panel"}
                onSelect={onSelect}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
