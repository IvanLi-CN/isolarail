import { useEffect, useRef, useState } from "react";
import type { DeviceTransport } from "../../app/device-runtime";
import {
  type CompanionBridge,
  tryBootstrapCompanionBridge,
} from "../../domain/companionBridge";
import type {
  DeviceInfoResponse,
  RebootResponse,
  Result,
  WifiConfigInput,
  WifiConfigResponse,
  WifiMutationResponse,
} from "../../domain/deviceApi";
import type { StoredDevice } from "../../domain/devices";
import {
  type FirmwareFlashProgress,
  flashWithLocalUsb,
  flashWithWebSerial,
  isWebSerialSupported,
  type SerialActivityEntry,
  WebSerialJsonlTransport,
} from "../../domain/hardwareConsole";
import { getLocalUsbDeviceLink } from "../../domain/localUsbLinks";
import {
  getWebSerialDeviceTransport,
  setWebSerialDeviceTransport,
  subscribeWebSerialDeviceLinks,
} from "../../domain/webSerialLinks";

const MAX_SERIAL_ACTIVITY_ITEMS = 24;

function unknown(value: string | null | undefined): string {
  if (value === null || value === undefined || value.trim().length === 0) {
    return "unknown";
  }
  return value;
}

function transportLabel(transport: DeviceTransport | null): string {
  if (transport === "http") {
    return "Wi-Fi / LAN";
  }
  if (transport === "local_usb") {
    return "Local USB";
  }
  if (transport === "web_serial") {
    return "Web Serial";
  }
  return "Not connected";
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

async function restoreWebSerialTransport(deviceId: string): Promise<void> {
  const transport = new WebSerialJsonlTransport();
  const current = getWebSerialDeviceTransport(deviceId);
  if (!current) {
    throw new Error("Web Serial transport is unavailable.");
  }
  const port = await current.takePortForExclusiveUse();
  await delay(250);
  await transport.connectToPort(port);
  setWebSerialDeviceTransport(deviceId, transport);
}

export function DeviceInfoPanel({
  mode = "hardware",
  device,
  transport,
  wifiManagementTransport,
  loadInfo,
  loadWifiConfig,
  saveWifiConfig,
  clearWifiConfig,
  rebootDevice,
  deleteDevice,
  serialActivityPreview = null,
}: {
  mode?: "hardware" | "info";
  device: StoredDevice;
  transport: DeviceTransport | null;
  wifiManagementTransport: DeviceTransport | null;
  loadInfo: () => Promise<Result<DeviceInfoResponse>>;
  loadWifiConfig: () => Promise<Result<WifiConfigResponse>>;
  saveWifiConfig: (
    input: WifiConfigInput,
  ) => Promise<Result<WifiMutationResponse>>;
  clearWifiConfig: () => Promise<Result<WifiMutationResponse>>;
  rebootDevice: () => Promise<Result<RebootResponse>>;
  deleteDevice: () => Promise<void>;
  serialActivityPreview?: SerialActivityEntry[] | null;
}) {
  const [info, setInfo] = useState<DeviceInfoResponse | null>(null);
  const [infoLoading, setInfoLoading] = useState(true);
  const [infoError, setInfoError] = useState<string | null>(null);
  const [wifiConfig, setWifiConfigState] = useState<WifiConfigResponse | null>(
    null,
  );
  const [wifiLoading, setWifiLoading] = useState(true);
  const [wifiBusy, setWifiBusy] = useState(false);
  const [wifiSsid, setWifiSsid] = useState("");
  const [wifiPsk, setWifiPsk] = useState("");
  const [wifiOpenNetwork, setWifiOpenNetwork] = useState(false);
  const [wifiStatus, setWifiStatus] = useState<string | null>(null);
  const [wifiError, setWifiError] = useState<string | null>(null);
  const [wifiRebootRequired, setWifiRebootRequired] = useState(false);
  const [wifiClearConfirmOpen, setWifiClearConfirmOpen] = useState(false);
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [firmwareFile, setFirmwareFile] = useState<File | null>(null);
  const [flashAddress, setFlashAddress] = useState("0x10000");
  const [flashBusy, setFlashBusy] = useState(false);
  const [flashStatus, setFlashStatus] = useState<string | null>(null);
  const [flashError, setFlashError] = useState<string | null>(null);
  const [flashProgress, setFlashProgress] =
    useState<FirmwareFlashProgress | null>(null);
  const [serialActivity, setSerialActivity] = useState<SerialActivityEntry[]>(
    [],
  );
  const visibleSerialActivity = serialActivityPreview ?? serialActivity;
  const loadInfoRef = useRef(loadInfo);
  const loadWifiConfigRef = useRef(loadWifiConfig);
  const wifiFormDirtyRef = useRef(false);
  const hasInfoRef = useRef(false);
  const hasWifiConfigRef = useRef(false);

  useEffect(() => {
    loadInfoRef.current = loadInfo;
  }, [loadInfo]);

  useEffect(() => {
    loadWifiConfigRef.current = loadWifiConfig;
  }, [loadWifiConfig]);

  useEffect(() => {
    if (device.id.length === 0) {
      return;
    }
    setInfo(null);
    hasInfoRef.current = false;
    setInfoLoading(true);
    setInfoError(null);
    setWifiConfigState(null);
    hasWifiConfigRef.current = false;
    setWifiLoading(true);
    setWifiSsid("");
    setWifiPsk("");
    setWifiOpenNetwork(false);
    setWifiStatus(null);
    setWifiError(null);
    setWifiRebootRequired(false);
    setWifiClearConfirmOpen(false);
    setDeleteConfirmOpen(false);
    setDeleteError(null);
    setSerialActivity([]);
    wifiFormDirtyRef.current = false;
  }, [device.id]);

  useEffect(() => {
    if (serialActivityPreview) {
      return;
    }
    if (transport !== "web_serial") {
      return;
    }

    let detachCurrent: (() => void) | null = null;
    const attach = (nextDeviceId = device.id) => {
      if (nextDeviceId !== device.id) {
        return;
      }
      detachCurrent?.();
      const current = getWebSerialDeviceTransport(device.id);
      if (!current) {
        detachCurrent = null;
        return;
      }
      detachCurrent = current.subscribeActivity((entry) => {
        setSerialActivity((prev) => {
          const next = [entry, ...prev];
          return next.slice(0, MAX_SERIAL_ACTIVITY_ITEMS);
        });
      });
    };

    attach();
    const detachLinks = subscribeWebSerialDeviceLinks((link) => {
      attach(link.deviceId);
    });
    return () => {
      detachCurrent?.();
      detachLinks();
    };
  }, [device.id, serialActivityPreview, transport]);

  useEffect(() => {
    let cancelled = false;
    let retryTimer: number | null = null;
    let retryCount = 0;

    const load = async () => {
      if (!transport) {
        setInfoLoading(false);
        setInfoError(null);
        return;
      }
      if (!hasInfoRef.current) {
        setInfoLoading(true);
      }
      if (retryTimer !== null) {
        window.clearTimeout(retryTimer);
        retryTimer = null;
      }
      const res = await loadInfoRef.current();
      if (cancelled) {
        return;
      }
      if (res.ok) {
        setInfo(res.value);
        hasInfoRef.current = true;
        setInfoError(null);
        retryCount = 0;
      } else {
        setInfoError(res.error.message);
        retryCount = Math.min(retryCount + 1, 5);
      }
      setInfoLoading(false);
      const delayMs = res.ok ? 15_000 : 800 * 2 ** Math.min(retryCount, 3);
      retryTimer = window.setTimeout(() => void load(), delayMs);
    };

    void load();
    return () => {
      cancelled = true;
      if (retryTimer !== null) {
        window.clearTimeout(retryTimer);
      }
    };
  }, [transport]);

  useEffect(() => {
    let cancelled = false;
    let retryTimer: number | null = null;

    const load = async () => {
      if (!transport) {
        setWifiLoading(false);
        setWifiError(null);
        return;
      }
      if (!hasWifiConfigRef.current) {
        setWifiLoading(true);
      }
      const res = await loadWifiConfigRef.current();
      if (cancelled) {
        return;
      }
      if (res.ok) {
        setWifiConfigState(res.value);
        hasWifiConfigRef.current = true;
        if (!wifiFormDirtyRef.current) {
          if (res.value.ssid !== undefined) {
            setWifiSsid(res.value.ssid);
          } else if (res.value.configured === false) {
            setWifiSsid("");
          }
        }
        setWifiError(null);
      } else {
        setWifiError(res.error.message);
      }
      setWifiLoading(false);
      const state = res.ok ? res.value.state : null;
      const delayMs = state === "connecting" ? 2_000 : 5_000;
      retryTimer = window.setTimeout(() => void load(), delayMs);
    };

    void load();
    return () => {
      cancelled = true;
      if (retryTimer !== null) {
        window.clearTimeout(retryTimer);
      }
    };
  }, [transport]);

  const deviceId = unknown(info?.device.device_id);
  const hostname = unknown(info?.device.hostname);
  const fqdn = unknown(info?.device.fqdn);
  const mac = unknown(info?.device.mac);
  const variant = unknown(info?.device.variant);
  const uptimeMs =
    info?.device.uptime_ms === undefined
      ? "unknown"
      : String(info.device.uptime_ms);

  const fwName = unknown(info?.device.firmware?.name);
  const fwVersion = unknown(info?.device.firmware?.version);
  const fwBuild = "unknown";
  const webSerialSupported = isWebSerialSupported();
  const firmwarePath =
    transport === "local_usb" || transport === "web_serial" ? transport : null;
  const firmwarePathLabel = transportLabel(transport);
  const firmwareUnavailableReason =
    transport === "http"
      ? "Firmware flashing is disabled over Wi-Fi/LAN because OTA is not implemented yet."
      : !firmwarePath
        ? "Connect this hub with Web Serial or Local USB to flash firmware."
        : null;

  const wifiRuntimeState = wifiConfig?.state ?? info?.device.wifi?.state;
  const wifiRuntimeIpv4 = wifiConfig?.ipv4 ?? info?.device.wifi?.ipv4;
  const wifiRuntimeIsStatic =
    wifiConfig?.is_static ?? info?.device.wifi?.is_static;
  const wifiState = unknown(wifiRuntimeState);
  const wifiIpv4 = unknown(wifiRuntimeIpv4 ?? undefined);
  const wifiIsStatic =
    wifiRuntimeIsStatic === undefined ? "unknown" : String(wifiRuntimeIsStatic);
  const wifiStorage = unknown(wifiConfig?.storage);
  const wifiAddress = unknown(wifiConfig?.address);
  const wifiConfigured =
    wifiConfig?.configured === undefined
      ? wifiConfig?.ssid
        ? "yes"
        : "unknown"
      : wifiConfig.configured
        ? "yes"
        : "no";
  const wifiPskConfigured =
    wifiConfig?.psk_configured === undefined
      ? "unknown"
      : wifiConfig.psk_configured
        ? "yes"
        : "no";
  const wifiCanManage =
    wifiManagementTransport === "web_serial" ||
    wifiManagementTransport === "local_usb";
  const wifiCanSubmit = wifiCanManage && !wifiBusy;
  const showInfoSkeleton = infoLoading && info === null && !infoError;
  const showWifiSkeleton = wifiLoading && wifiConfig === null && !wifiError;

  const saveWifi = async () => {
    const nextPsk = wifiOpenNetwork ? "" : wifiPsk;
    if (!wifiCanManage) {
      setWifiError(
        "Connect with Web Serial or Local USB before changing Wi-Fi configuration.",
      );
      return;
    }
    const validationError = validateWifiInput(wifiSsid, nextPsk);
    if (validationError) {
      setWifiError(validationError);
      return;
    }
    if (
      wifiConfig?.psk_configured === true &&
      nextPsk.length === 0 &&
      !wifiOpenNetwork
    ) {
      setWifiError(
        "Enter the Wi-Fi PSK again before saving, or choose Open network to replace the stored PSK.",
      );
      return;
    }

    setWifiBusy(true);
    setWifiError(null);
    setWifiStatus(null);
    try {
      const res = await saveWifiConfig({
        ssid: wifiSsid,
        psk: nextPsk,
      });
      if (res.ok) {
        setWifiConfigState((prev) => ({
          ...prev,
          storage: prev?.storage ?? "eeprom",
          address: prev?.address ?? "0x50",
          configured: true,
          ssid: wifiSsid,
          psk_configured: nextPsk.length > 0,
          state: res.value.reboot_required ? prev?.state : "connecting",
          ipv4: res.value.reboot_required ? prev?.ipv4 : null,
          is_static: prev?.is_static,
        }));
        setWifiPsk("");
        setWifiOpenNetwork(false);
        wifiFormDirtyRef.current = false;
        setWifiRebootRequired(res.value.reboot_required);
        setWifiStatus(
          res.value.reboot_required
            ? "Wi-Fi configuration saved. Reboot this hub to apply it."
            : "Wi-Fi configuration saved and applying now.",
        );
        return;
      }
      setWifiError(res.error.message);
    } finally {
      setWifiBusy(false);
    }
  };

  const clearWifiNow = async () => {
    setWifiBusy(true);
    setWifiError(null);
    setWifiStatus(null);
    setWifiClearConfirmOpen(false);
    try {
      const res = await clearWifiConfig();
      if (res.ok) {
        setWifiConfigState((prev) => ({
          storage: prev?.storage ?? "eeprom",
          address: prev?.address ?? "0x50",
          configured: false,
          psk_configured: false,
          state: res.value.reboot_required ? prev?.state : "idle",
          ipv4: res.value.reboot_required ? prev?.ipv4 : null,
          is_static: res.value.reboot_required ? prev?.is_static : false,
        }));
        setWifiSsid("");
        setWifiPsk("");
        setWifiOpenNetwork(false);
        wifiFormDirtyRef.current = false;
        setWifiRebootRequired(res.value.reboot_required);
        setWifiStatus(
          res.value.reboot_required
            ? "Wi-Fi configuration cleared. Reboot this hub to apply it."
            : "Wi-Fi configuration cleared and Wi-Fi is stopping.",
        );
        return;
      }
      setWifiError(res.error.message);
    } finally {
      setWifiBusy(false);
    }
  };

  const rebootForWifi = async () => {
    if (!wifiCanManage) {
      setWifiError(
        "Connect with Web Serial or Local USB before applying Wi-Fi configuration changes.",
      );
      return;
    }
    setWifiBusy(true);
    setWifiError(null);
    try {
      const res = await rebootDevice();
      if (res.ok) {
        setWifiRebootRequired(false);
        setWifiStatus("Reboot accepted. The hub may disconnect briefly.");
        return;
      }
      setWifiError(res.error.message);
    } finally {
      setWifiBusy(false);
    }
  };

  const confirmDeleteDevice = async () => {
    setDeleteBusy(true);
    setDeleteError(null);
    try {
      await deleteDevice();
    } catch (err) {
      setDeleteError(
        err instanceof Error ? err.message : "Could not delete this hub.",
      );
      setDeleteBusy(false);
    }
  };

  const resolveLocalUsbFlashPort = async (): Promise<{
    agent: CompanionBridge;
    portPath: string;
  }> => {
    const agent = await tryBootstrapCompanionBridge();
    if (!agent) {
      throw new Error("Local companion is not running.");
    }
    const linkedPort = getLocalUsbDeviceLink(device.id);
    if (!linkedPort) {
      throw new Error(
        "Reconnect this hub with Local USB from Add Device before flashing firmware.",
      );
    }
    return { agent, portPath: linkedPort };
  };

  const flashFirmware = async () => {
    if (!firmwarePath) {
      setFlashError(
        "Firmware flashing requires Web Serial or Local USB. OTA over Wi-Fi/LAN is not implemented yet.",
      );
      return;
    }
    if (!firmwareFile) {
      setFlashError("Select a firmware .bin first.");
      return;
    }
    const address = parseFlashAddress(flashAddress);
    if (address === null) {
      setFlashError("Enter a valid flash address, for example 0x10000.");
      return;
    }

    setFlashBusy(true);
    setFlashError(null);
    setFlashProgress(null);
    try {
      if (firmwarePath === "local_usb") {
        setFlashStatus("Using the linked Local USB port...");
        const resolved = await resolveLocalUsbFlashPort();
        setFlashStatus("Writing firmware over Local USB...");
        const output = await flashWithLocalUsb(
          resolved.agent,
          resolved.portPath,
          firmwareFile,
          address,
          {
            deviceId: device.id,
            mac: info?.device.mac,
          },
        );
        setFlashStatus(output || "Firmware update completed over Local USB.");
        return;
      }

      if (!webSerialSupported) {
        throw new Error("Web Serial is not supported by this browser.");
      }
      const currentTransport = getWebSerialDeviceTransport(device.id);
      if (!currentTransport) {
        throw new Error(
          "Connect this hub with Web Serial before flashing firmware.",
        );
      }
      setFlashStatus(
        "Preparing connected Web Serial port for firmware update...",
      );
      const port = await currentTransport.takePortForExclusiveUse();
      let firmwareWritten = false;
      try {
        setFlashStatus("Writing firmware over the current Web Serial link...");
        await flashWithWebSerial(port, firmwareFile, address, setFlashProgress);
        firmwareWritten = true;
        setFlashStatus("Firmware update completed over Web Serial.");
      } finally {
        try {
          if (firmwareWritten) {
            await restoreWebSerialTransport(device.id);
          } else {
            const restoredTransport = new WebSerialJsonlTransport();
            await delay(250);
            await restoredTransport.connectToPort(port);
            setWebSerialDeviceTransport(device.id, restoredTransport);
          }
        } catch {
          // The hub may still be re-enumerating after a successful flash.
        }
      }
    } catch (err) {
      setFlashError(
        err instanceof Error ? err.message : "Firmware update failed.",
      );
    } finally {
      setFlashBusy(false);
    }
  };

  const showHardwareControls = mode === "hardware";

  return (
    <div className="flex flex-col gap-6" data-testid="device-info">
      <div className="iso-card min-h-[168px] rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
        <div className="text-[16px] font-bold leading-5">Identity</div>
        <div className="mt-[14px] grid grid-cols-1 gap-6 md:grid-cols-[minmax(0,564px)_minmax(0,1fr)]">
          <div className="flex flex-col gap-[10px]">
            <IdentityRow
              label="device_id"
              value={deviceId}
              loading={showInfoSkeleton}
            />
            <IdentityRow
              label="hostname"
              value={hostname}
              loading={showInfoSkeleton}
            />
            <IdentityRow label="fqdn" value={fqdn} loading={showInfoSkeleton} />
            <IdentityRow label="mac" value={mac} loading={showInfoSkeleton} />
          </div>
          <div className="flex flex-col gap-[10px]">
            <IdentityRow
              label="variant"
              value={variant}
              loading={showInfoSkeleton}
              narrow
            />
            <IdentityRow
              label="uptime_ms"
              value={uptimeMs}
              loading={showInfoSkeleton}
              wide
            />
          </div>
        </div>
        {infoError ? (
          <div
            className="mt-4 rounded-[10px] border border-[var(--error)] px-3 py-2 text-[12px] font-semibold leading-5 text-[var(--error)]"
            role="alert"
          >
            {transportLabel(transport)} info failed: {infoError}
          </div>
        ) : null}
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <InfoCard
          title="Firmware"
          rows={[
            ["name", fwName],
            ["version", fwVersion],
            ["build", fwBuild],
          ]}
          loading={showInfoSkeleton}
        />
        <InfoCard
          title="Wi-Fi runtime"
          rows={[
            ["state", wifiState],
            ["ipv4", wifiIpv4],
            ["is_static", wifiIsStatic],
          ]}
          loading={showWifiSkeleton}
        />
      </div>

      <div className="iso-card rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div className="min-w-0">
            <div className="text-[16px] font-bold leading-5">
              Wi-Fi configuration
            </div>
            <div className="mt-2 text-[12px] font-semibold leading-5 text-[var(--muted)]">
              Credentials are stored in EEPROM `M24C64@0x50`. Wi-Fi/LAN can read
              status, but writes require a USB-capable transport.
            </div>
          </div>
          <div className="flex flex-col gap-2 sm:flex-row">
            <div className="flex min-h-8 items-center rounded-[10px] border border-[var(--border)] bg-[var(--panel-2)] px-3 text-[12px] font-bold text-[var(--muted)]">
              Current: {transportLabel(transport)}
            </div>
            <div className="flex min-h-8 items-center rounded-[10px] border border-[var(--border)] bg-[var(--panel-2)] px-3 text-[12px] font-bold text-[var(--muted)]">
              Manage: {transportLabel(wifiManagementTransport)}
            </div>
          </div>
        </div>

        <div className="mt-5 grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
          <InfoPill
            label="state"
            value={wifiState}
            loading={showWifiSkeleton}
          />
          <InfoPill label="ipv4" value={wifiIpv4} loading={showWifiSkeleton} />
          <InfoPill
            label="static"
            value={wifiIsStatic}
            loading={showWifiSkeleton}
          />
          <InfoPill
            label="configured"
            value={wifiConfigured}
            loading={showWifiSkeleton}
          />
          <InfoPill
            label="psk"
            value={wifiPskConfigured}
            loading={showWifiSkeleton}
          />
          <InfoPill
            label="storage"
            value={wifiStorage}
            loading={showWifiSkeleton}
          />
          <InfoPill
            label="address"
            value={wifiAddress}
            loading={showWifiSkeleton}
          />
        </div>

        {showHardwareControls ? (
          <>
            <div className="mt-5 grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
              <label className="form-control min-w-0">
                <div className="label px-0 pb-1 pt-0">
                  <span className="label-text text-[12px] font-bold text-[var(--muted)]">
                    SSID
                  </span>
                </div>
                <input
                  className="input input-sm w-full font-mono"
                  autoComplete="off"
                  value={wifiSsid}
                  disabled={!wifiCanManage || wifiBusy}
                  onChange={(event) => {
                    wifiFormDirtyRef.current = true;
                    setWifiSsid(event.target.value);
                  }}
                  placeholder="Network name"
                />
              </label>
              <div className="min-w-0">
                <label className="form-control min-w-0">
                  <div className="label px-0 pb-1 pt-0">
                    <span className="label-text text-[12px] font-bold text-[var(--muted)]">
                      PSK
                    </span>
                  </div>
                  <input
                    className="input input-sm w-full font-mono"
                    type="password"
                    autoComplete="new-password"
                    value={wifiPsk}
                    disabled={!wifiCanManage || wifiBusy || wifiOpenNetwork}
                    onChange={(event) => {
                      wifiFormDirtyRef.current = true;
                      setWifiPsk(event.target.value);
                      if (event.target.value.length > 0) {
                        setWifiOpenNetwork(false);
                      }
                    }}
                    placeholder="Blank means open network"
                  />
                </label>
                <label className="mt-2 flex min-h-6 items-center gap-2 text-[12px] font-semibold text-[var(--muted)]">
                  <input
                    className="checkbox checkbox-xs"
                    type="checkbox"
                    checked={wifiOpenNetwork}
                    disabled={!wifiCanManage || wifiBusy}
                    onChange={(event) => {
                      const checked = event.target.checked;
                      wifiFormDirtyRef.current = true;
                      setWifiOpenNetwork(checked);
                      if (checked) {
                        setWifiPsk("");
                      }
                    }}
                  />
                  <span>Open network (no PSK)</span>
                </label>
              </div>
            </div>

            <div className="mt-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
              <div className="text-[12px] font-semibold leading-5 text-[var(--muted)]">
                {wifiCanManage
                  ? "Existing PSK is never shown. Re-enter it before saving a secured network, or choose Open network to replace it."
                  : "Wi-Fi/LAN is read-only for Wi-Fi settings. Connect with Web Serial or Local USB to change credentials."}
              </div>
              <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 md:min-w-[260px]">
                <button
                  className="btn btn-primary btn-sm min-h-10"
                  type="button"
                  disabled={!wifiCanSubmit}
                  onClick={() => void saveWifi()}
                >
                  {wifiBusy ? "Saving..." : "Save Wi-Fi"}
                </button>
                <button
                  className="btn btn-outline btn-sm min-h-10"
                  type="button"
                  disabled={!wifiCanSubmit}
                  onClick={() => setWifiClearConfirmOpen(true)}
                >
                  Clear
                </button>
                {wifiRebootRequired ? (
                  <button
                    className="btn btn-outline btn-sm min-h-10"
                    type="button"
                    disabled={!wifiCanSubmit}
                    onClick={() => void rebootForWifi()}
                  >
                    Reboot
                  </button>
                ) : null}
              </div>
            </div>
          </>
        ) : (
          <div className="mt-5 rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-4 py-3 text-[12px] font-semibold text-[var(--muted)]">
            Settings changes are disabled on this page. Switch to Settings to
            save credentials, clear EEPROM Wi-Fi state, or reboot after a Wi-Fi
            update.
          </div>
        )}

        {wifiStatus ? (
          <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-4 py-3 text-[12px] font-semibold text-[var(--muted)]">
            {wifiStatus}
          </div>
        ) : null}

        {wifiError ? (
          <div
            className="mt-4 rounded-[12px] border border-[var(--error)] bg-[var(--panel)] px-4 py-3 text-[12px] font-semibold text-[var(--error)]"
            role="alert"
          >
            Wi-Fi configuration failed: {wifiError}
          </div>
        ) : null}
      </div>

      {showHardwareControls ? (
        <div className="iso-card rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
            <div className="min-w-0">
              <div className="text-[16px] font-bold leading-5">
                Firmware update
              </div>
              <div className="mt-2 text-[12px] font-semibold leading-5 text-[var(--muted)]">
                Flash an ESP32-S3 application image at `0x10000`.
              </div>
            </div>
            <div className="flex min-h-8 items-center rounded-[10px] border border-[var(--border)] bg-[var(--panel-2)] px-3 text-[12px] font-bold text-[var(--muted)]">
              Current: {firmwarePathLabel}
            </div>
          </div>

          {firmwareUnavailableReason ? (
            <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-4 py-3 text-[12px] font-semibold text-[var(--muted)]">
              {firmwareUnavailableReason}
            </div>
          ) : null}

          <div className="mt-5 grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_132px]">
            <input
              className="file-input file-input-sm w-full"
              type="file"
              accept=".bin,application/octet-stream"
              onChange={(event) =>
                setFirmwareFile(event.currentTarget.files?.[0] ?? null)
              }
            />
            <input
              className="input input-sm w-full font-mono"
              aria-label="Flash address"
              value={flashAddress}
              onChange={(event) => setFlashAddress(event.target.value)}
            />
          </div>

          {firmwarePath === "web_serial" && !webSerialSupported ? (
            <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-4 py-3 text-[12px] font-semibold text-[var(--warning)]">
              This browser does not expose Web Serial. Use Chrome/Edge or Local
              USB.
            </div>
          ) : null}

          {flashProgress ? (
            <div className="mt-4 text-[12px] font-semibold text-[var(--muted)]">
              {flashProgress.message}
              {flashProgress.total
                ? ` ${Math.round(((flashProgress.written ?? 0) / flashProgress.total) * 100)}%`
                : ""}
            </div>
          ) : null}

          {flashStatus ? (
            <div className="mt-4 text-[12px] font-semibold text-[var(--muted)]">
              {flashStatus}
            </div>
          ) : null}

          {flashError ? (
            <div
              className="mt-4 rounded-[12px] border border-[var(--error)] bg-[var(--panel)] px-4 py-3 text-[12px] font-semibold text-[var(--error)]"
              role="alert"
            >
              {flashError}
            </div>
          ) : null}

          <button
            className="btn btn-primary mt-5 h-11 w-full justify-center"
            type="button"
            disabled={
              flashBusy ||
              !firmwareFile ||
              !firmwarePath ||
              (firmwarePath === "web_serial" && !webSerialSupported)
            }
            onClick={() => void flashFirmware()}
          >
            {flashBusy ? "Updating..." : "Flash firmware"}
          </button>
        </div>
      ) : null}

      <div className="iso-card rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div className="min-w-0">
            <div className="text-[16px] font-bold leading-5">
              Serial activity
            </div>
            <div className="mt-2 text-[12px] font-semibold leading-5 text-[var(--muted)]">
              Web Serial shows live USB CDC activity, including JSONL traffic,
              boot lines, and defmt-like binary frames.
            </div>
          </div>
          <div className="flex min-h-8 items-center rounded-[10px] border border-[var(--border)] bg-[var(--panel-2)] px-3 text-[12px] font-bold text-[var(--muted)]">
            Current: {transportLabel(transport)}
          </div>
        </div>

        {transport !== "web_serial" ? (
          <div className="mt-5 rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-4 py-3 text-[12px] font-semibold text-[var(--muted)]">
            Live serial activity appears when this device is connected with Web
            Serial. Local USB bridge and Wi-Fi/LAN still record session traces
            through the local companion.
          </div>
        ) : visibleSerialActivity.length === 0 ? (
          <div className="mt-5 rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-4 py-3 text-[12px] font-semibold text-[var(--muted)]">
            Waiting for USB CDC traffic. Request device info, save Wi-Fi, or
            reconnect the port to populate this timeline.
          </div>
        ) : (
          <div className="mt-5 overflow-hidden rounded-[14px] border border-[var(--border)] bg-[var(--panel-2)]">
            <div className="grid grid-cols-[88px_88px_minmax(0,1fr)] gap-3 border-b border-[var(--border)] px-4 py-3 text-[11px] font-bold uppercase tracking-[0.08em] text-[var(--muted)]">
              <div>Kind</div>
              <div>Request</div>
              <div>Summary</div>
            </div>
            <div className="divide-y divide-[var(--border)]">
              {visibleSerialActivity.map((entry) => (
                <SerialActivityRow entry={entry} key={entry.id} />
              ))}
            </div>
          </div>
        )}
      </div>

      {wifiClearConfirmOpen ? (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/45 px-4 py-6"
          role="presentation"
        >
          <div
            className="w-full max-w-[440px] rounded-[14px] border border-[var(--border)] bg-[var(--panel)] p-5 shadow-2xl"
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="wifi-clear-title"
            aria-describedby="wifi-clear-description"
          >
            <div
              id="wifi-clear-title"
              className="text-[15px] font-bold text-[var(--text)]"
            >
              Clear stored Wi-Fi configuration?
            </div>
            <div
              id="wifi-clear-description"
              className="mt-3 text-[13px] font-semibold leading-6 text-[var(--muted)]"
            >
              The hub will forget the saved SSID and PSK, and Wi-Fi will stop
              immediately.
            </div>
            <div className="mt-5 grid grid-cols-2 gap-3">
              <button
                className="btn btn-outline btn-sm min-h-10 justify-center"
                type="button"
                disabled={wifiBusy}
                onClick={() => setWifiClearConfirmOpen(false)}
              >
                Cancel
              </button>
              <button
                className="btn btn-primary btn-sm min-h-10 justify-center"
                type="button"
                disabled={wifiBusy}
                onClick={() => void clearWifiNow()}
              >
                Clear Wi-Fi
              </button>
            </div>
          </div>
        </div>
      ) : null}

      <div className="iso-card h-[156px] rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
        <div className="text-[16px] font-bold leading-5">Notes</div>
        <div className="mt-[14px] space-y-[6px] text-[14px] font-medium leading-5">
          <div>- Missing fields render as “unknown”</div>
          <div>- Connection: offline when last ok ≥ 10s</div>
          <div>- Replug means controlled power-cycle on this hardware</div>
        </div>
      </div>

      <div className="iso-card rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
        <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div className="min-w-0">
            <div className="text-[16px] font-bold leading-5">Saved device</div>
            <div className="mt-2 text-[12px] font-semibold leading-5 text-[var(--muted)]">
              Remove this hub from the local device list.
            </div>
          </div>
          {showHardwareControls ? (
            <button
              className="btn btn-outline btn-sm min-h-10 justify-center border-[var(--error)] text-[var(--error)]"
              type="button"
              onClick={() => {
                setDeleteError(null);
                setDeleteConfirmOpen(true);
              }}
            >
              Delete device
            </button>
          ) : null}
        </div>
        {deleteError ? (
          <div
            className="mt-4 rounded-[12px] border border-[var(--error)] bg-[var(--panel)] px-4 py-3 text-[12px] font-semibold text-[var(--error)]"
            role="alert"
          >
            {deleteError}
          </div>
        ) : null}
      </div>

      {deleteConfirmOpen ? (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/45 px-4 py-6"
          role="presentation"
        >
          <div
            className="w-full max-w-[460px] rounded-[14px] border border-[var(--border)] bg-[var(--panel)] p-5 shadow-2xl"
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="device-delete-title"
            aria-describedby="device-delete-description"
          >
            <div
              id="device-delete-title"
              className="text-[15px] font-bold text-[var(--text)]"
            >
              Delete this saved device?
            </div>
            <div
              id="device-delete-description"
              className="mt-3 text-[13px] font-semibold leading-6 text-[var(--muted)]"
            >
              This only removes the local saved profile for {device.name}. It
              does not change hardware settings on the hub.
            </div>
            <div className="mt-5 grid grid-cols-2 gap-3">
              <button
                className="btn btn-outline btn-sm min-h-10 justify-center"
                type="button"
                disabled={deleteBusy}
                onClick={() => setDeleteConfirmOpen(false)}
              >
                Cancel
              </button>
              <button
                className="btn btn-primary btn-sm min-h-10 justify-center"
                type="button"
                disabled={deleteBusy}
                onClick={() => void confirmDeleteDevice()}
              >
                {deleteBusy ? "Deleting..." : "Delete device"}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      <div className="text-[12px] font-semibold text-[var(--muted)]">
        {mode === "hardware"
          ? "Settings focuses on EEPROM-backed Wi-Fi provisioning, firmware flashing, and local maintenance."
          : "Info summarizes firmware, connectivity, and saved-device metadata for this four-port hub."}
      </div>
    </div>
  );
}

function SerialActivityRow({ entry }: { entry: SerialActivityEntry }) {
  return (
    <div className="grid grid-cols-[88px_88px_minmax(0,1fr)] gap-3 px-4 py-3 text-[12px] font-semibold">
      <div className="font-mono uppercase text-[var(--muted)]">
        {entry.kind}
      </div>
      <div className="font-mono text-[var(--muted)]">
        {entry.requestId ?? "—"}
      </div>
      <div className="min-w-0">
        <div className="truncate">{entry.summary}</div>
        <div className="mt-1 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[11px] text-[var(--muted)]">
          {entry.payload}
        </div>
      </div>
    </div>
  );
}

function IdentityRow({
  label,
  value,
  loading = false,
  narrow = false,
  wide = false,
}: {
  label: string;
  value: string;
  loading?: boolean;
  narrow?: boolean;
  wide?: boolean;
}) {
  const width = wide ? "w-[90px]" : narrow ? "w-[70px]" : "w-[84px]";
  return (
    <div className="flex min-w-0 items-center leading-[14px]">
      <div
        className={`${width} text-[12px] font-semibold leading-[14px] text-[var(--muted)]`}
      >
        {label}
      </div>
      <div className="min-w-0 truncate font-mono text-[12px] font-semibold">
        {loading ? <SkeletonLine width="w-[128px]" /> : value}
      </div>
    </div>
  );
}

function InfoCard({
  title,
  rows,
  loading = false,
}: {
  title: string;
  rows: Array<[label: string, value: string]>;
  loading?: boolean;
}) {
  return (
    <div className="iso-card h-[152px] rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
      <div className="text-[16px] font-bold leading-5">{title}</div>
      <div className="mt-[14px] flex flex-col gap-[10px] leading-[14px]">
        {rows.map(([label, value]) => (
          <div className="flex min-w-0 items-center" key={label}>
            <div className="w-[64px] text-[12px] font-semibold text-[var(--muted)]">
              {label}
            </div>
            <div className="min-w-0 truncate font-mono text-[12px] font-semibold">
              {loading ? <SkeletonLine width="w-[112px]" /> : value}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function parseFlashAddress(value: string): number | null {
  const trimmed = value.trim();
  if (!/^0x[0-9a-f]+$/i.test(trimmed)) {
    return null;
  }
  const parsed = Number.parseInt(trimmed, 16);
  return Number.isFinite(parsed) ? parsed : null;
}

function InfoPill({
  label,
  value,
  loading = false,
}: {
  label: string;
  value: string;
  loading?: boolean;
}) {
  return (
    <div className="min-w-0 rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-3 py-2">
      <div className="text-[11px] font-bold uppercase leading-4 text-[var(--muted)]">
        {label}
      </div>
      <div className="min-w-0 truncate font-mono text-[12px] font-semibold leading-5">
        {loading ? <SkeletonLine width="w-[96px]" /> : value}
      </div>
    </div>
  );
}

function SkeletonLine({ width }: { width: string }) {
  return (
    <span
      aria-hidden="true"
      className={`iso-skeleton-line inline-block h-[13px] max-w-full rounded-[6px] align-middle ${width}`}
    />
  );
}

function validateWifiInput(ssid: string, psk: string): string | null {
  const ssidBytes = utf8ByteLength(ssid);
  if (ssidBytes === 0) {
    return "SSID is required.";
  }
  if (ssidBytes > 32) {
    return "SSID must be 32 bytes or fewer.";
  }

  const pskBytes = utf8ByteLength(psk);
  if (pskBytes > 0 && pskBytes < 8) {
    return "PSK must be blank or at least 8 bytes.";
  }
  if (pskBytes > 64) {
    return "PSK must be 64 bytes or fewer.";
  }
  return null;
}

function utf8ByteLength(value: string): number {
  return new TextEncoder().encode(value).length;
}
