import { useEffect, useMemo, useReducer, useRef, useState } from "react";
import { useNavigate } from "react-router";
import {
  agentFetch,
  type CompanionBridge,
  tryBootstrapCompanionBridge,
} from "../../domain/companionBridge";
import { getDeviceInfo } from "../../domain/deviceApi";
import type {
  AddDeviceInput,
  AddDeviceValidationResult,
} from "../../domain/devices";
import { loadStoredDevices } from "../../domain/devices";
import type { DiscoveredDevice, LanCandidate } from "../../domain/discovery";
import {
  createInitialDiscoverySnapshot,
  mergeDiscoveredDevice,
  parseCidr,
  parseDiscoveredDeviceFromApiInfo,
  reduceDiscoverySnapshot,
} from "../../domain/discovery";
import {
  filterEsp32SerialPorts,
  isWebSerialSupported,
  listLocalUsbSerialPorts,
  type SerialPortInfo,
  WebSerialJsonlTransport,
} from "../../domain/hardwareConsole";
import { announceLocalUsbDeviceLink } from "../../domain/localUsbLinks";
import { announceNetworkDeviceLink } from "../../domain/networkLinks";
import { announceWebSerialDeviceLink } from "../../domain/webSerialLinks";
import { DeviceDiscoveryPanel } from "../panels/DeviceDiscoveryPanel";

import {
  hydrateInitialUsbLog,
  InlineAddError,
  isIsoHubDeviceInfo,
  normalizeDeviceId,
  parseUsbInfoEnvelope,
  readLocalUsbInfo,
  readWebSerialInfo,
  shortIdFromMac,
  type UsbDeviceInfo,
  type UsbLogEntry,
  usbInfoMatchesHttpInfo,
} from "./AddDeviceDialog.helpers";

type AddDeviceMethod = "wifi" | "web_serial" | "local_usb";

export type AddDeviceDialogProps = {
  open: boolean;
  initialMethod?: AddDeviceMethod;
  initialUsbLog?: Array<Omit<UsbLogEntry, "id">>;
  existingDeviceIds?: string[];
  existingDeviceBaseUrls?: string[];
  existingDeviceNamesById?: Record<string, string>;
  onClose: () => void;
  onCreate: (input: AddDeviceInput) => Promise<AddDeviceValidationResult>;
  onUpsert: (input: AddDeviceInput) => Promise<AddDeviceValidationResult>;
};

export function AddDeviceDialog({
  open,
  initialMethod = "wifi",
  initialUsbLog,
  existingDeviceIds,
  existingDeviceBaseUrls,
  existingDeviceNamesById,
  onClose,
  onCreate,
  onUpsert,
}: AddDeviceDialogProps) {
  const navigate = useNavigate();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const devicesCountRef = useRef(0);
  const ipScanExpandedRef = useRef(false);
  const openRef = useRef(open);
  const methodRef = useRef<AddDeviceMethod>(initialMethod);
  const usbRunIdRef = useRef(0);
  const [method, setMethod] = useState<AddDeviceMethod>(initialMethod);
  const [addError, setAddError] = useState<string | null>(null);
  const [usbBusy, setUsbBusy] = useState(false);
  const [usbStatus, setUsbStatus] = useState<string | null>(null);
  const [usbLog, setUsbLog] = useState<UsbLogEntry[]>(() =>
    hydrateInitialUsbLog(initialUsbLog),
  );
  const [localUsbPorts, setLocalUsbPorts] = useState<SerialPortInfo[]>([]);
  const [selectedLocalUsbPort, setSelectedLocalUsbPort] = useState("");
  const [discoveryPanelKey, setDiscoveryPanelKey] = useState(0);

  const ids = useMemo(() => existingDeviceIds ?? [], [existingDeviceIds]);
  const baseUrls = useMemo(
    () =>
      existingDeviceBaseUrls ??
      (open ? loadStoredDevices().map((d) => d.baseUrl) : []),
    [existingDeviceBaseUrls, open],
  );

  const [snapshot, dispatch] = useReducer(
    reduceDiscoverySnapshot,
    createInitialDiscoverySnapshot({
      status: "unavailable",
      autoExpandAfterMs: 30_000,
    }),
  );

  const agentRef = useRef<CompanionBridge | null>(null);
  const agentPollRef = useRef<number | null>(null);

  const scanRunIdRef = useRef(0);
  const usbLogSeqRef = useRef(1);

  const appendUsbLog = (
    message: string,
    tone: UsbLogEntry["tone"] = "info",
  ) => {
    const entry = { id: usbLogSeqRef.current, message, tone };
    usbLogSeqRef.current += 1;
    setUsbLog((prev) => [...prev.slice(-7), entry]);
  };

  const setUsbStep = (message: string, tone: UsbLogEntry["tone"] = "info") => {
    setUsbStatus(message);
    appendUsbLog(message, tone);
  };

  useEffect(() => {
    openRef.current = open;
  }, [open]);

  useEffect(() => {
    methodRef.current = method;
  }, [method]);

  useEffect(() => {
    devicesCountRef.current = snapshot.devices.length;
    ipScanExpandedRef.current = snapshot.ipScan?.expanded ?? false;
  }, [snapshot.devices.length, snapshot.ipScan?.expanded]);

  useEffect(() => {
    const el = dialogRef.current;
    if (!el) {
      return;
    }
    if (open) {
      if (!el.open) {
        el.showModal();
      }
      setAddError(null);
      setUsbBusy(false);
      setUsbStatus(null);
      setUsbLog(hydrateInitialUsbLog(initialUsbLog));
      setLocalUsbPorts([]);
      setSelectedLocalUsbPort("");
      methodRef.current = initialMethod;
      usbRunIdRef.current += 1;
      setMethod(initialMethod);
      setDiscoveryPanelKey((v) => v + 1);
      dispatch({ type: "reset", status: "unavailable" });
      return;
    }

    scanRunIdRef.current += 1;
    usbRunIdRef.current += 1;
    dispatch({ type: "scan_cancelled" });
    agentRef.current = null;
    if (agentPollRef.current) {
      window.clearInterval(agentPollRef.current);
      agentPollRef.current = null;
    }
    if (el.open) {
      el.close();
    }
  }, [initialMethod, initialUsbLog, open]);

  useEffect(() => {
    if (!open) {
      return;
    }

    void (async () => {
      const agent = await tryBootstrapCompanionBridge();
      agentRef.current = agent;
      if (!agent) {
        dispatch({ type: "reset", status: "unavailable" });
        return;
      }

      dispatch({ type: "reset", status: "scanning" });
      await agentFetch(agent, "/api/v1/discovery/refresh", {
        method: "POST",
        body: JSON.stringify({}),
      });

      if (agentPollRef.current) {
        window.clearInterval(agentPollRef.current);
        agentPollRef.current = null;
      }

      agentPollRef.current = window.setInterval(() => {
        void (async () => {
          const current = agentRef.current;
          if (!current) {
            return;
          }
          const res = await agentFetch(
            current,
            "/api/v1/discovery/snapshot",
            {},
          );
          if (!res.ok) {
            dispatch({
              type: "set_error",
              error:
                res.status === 401 || res.status === 403
                  ? "Local companion authorization failed."
                  : "Local companion unavailable.",
            });
            return;
          }
          const value = (await res.json()) as unknown;
          if (!value || typeof value !== "object") {
            return;
          }
          const obj = value as Record<string, unknown>;
          const devices = Array.isArray(obj.devices) ? obj.devices : null;
          const mode =
            obj.mode === "service" || obj.mode === "scan"
              ? obj.mode
              : "service";
          const status =
            obj.status === "idle" ||
            obj.status === "scanning" ||
            obj.status === "ready" ||
            obj.status === "unavailable"
              ? obj.status
              : "unavailable";
          const error = typeof obj.error === "string" ? obj.error : undefined;
          const scan =
            obj.scan && typeof obj.scan === "object"
              ? (obj.scan as Record<string, unknown>)
              : undefined;
          const ipScan =
            obj.ipScan && typeof obj.ipScan === "object"
              ? (obj.ipScan as Record<string, unknown>)
              : undefined;

          const scanShape =
            scan &&
            typeof scan.cidr === "string" &&
            typeof scan.done === "number" &&
            typeof scan.total === "number"
              ? { cidr: scan.cidr, done: scan.done, total: scan.total }
              : undefined;

          const defaultCidr =
            ipScan && typeof ipScan.defaultCidr === "string"
              ? ipScan.defaultCidr
              : undefined;
          const candidatesRaw =
            ipScan && Array.isArray(ipScan.candidates)
              ? ipScan.candidates
              : null;
          const candidates = candidatesRaw
            ? candidatesRaw.reduce<LanCandidate[]>((acc, item) => {
                if (!item || typeof item !== "object") {
                  return acc;
                }
                const c = item as Record<string, unknown>;
                if (typeof c.cidr !== "string") {
                  return acc;
                }

                const parsed: LanCandidate = { cidr: c.cidr };
                if (typeof c.label === "string") {
                  parsed.label = c.label;
                }
                if (typeof c.interface === "string") {
                  parsed.interface = c.interface;
                }
                if (typeof c.ipv4 === "string") {
                  parsed.ipv4 = c.ipv4;
                }
                if (typeof c.primary === "boolean") {
                  parsed.primary = c.primary;
                }
                acc.push(parsed);
                return acc;
              }, [])
            : undefined;

          const parsedDevices: DiscoveredDevice[] = [];
          if (devices) {
            for (const item of devices) {
              if (!item || typeof item !== "object") {
                continue;
              }
              const d = item as Record<string, unknown>;
              if (typeof d.baseUrl !== "string") {
                continue;
              }
              parsedDevices.push({
                baseUrl: d.baseUrl,
                device_id:
                  typeof d.device_id === "string" ? d.device_id : undefined,
                hostname:
                  typeof d.hostname === "string" ? d.hostname : undefined,
                fqdn: typeof d.fqdn === "string" ? d.fqdn : undefined,
                ipv4: typeof d.ipv4 === "string" ? d.ipv4 : undefined,
                variant: typeof d.variant === "string" ? d.variant : undefined,
                firmware:
                  d.firmware &&
                  typeof d.firmware === "object" &&
                  typeof (d.firmware as Record<string, unknown>).name ===
                    "string" &&
                  typeof (d.firmware as Record<string, unknown>).version ===
                    "string"
                    ? {
                        name: (d.firmware as Record<string, unknown>)
                          .name as string,
                        version: (d.firmware as Record<string, unknown>)
                          .version as string,
                      }
                    : undefined,
                last_seen_at:
                  typeof d.last_seen_at === "string"
                    ? d.last_seen_at
                    : undefined,
              });
            }
          }

          // Preserve local reducer dedup semantics (device_id preferred).
          let merged: DiscoveredDevice[] = [];
          for (const d of parsedDevices) {
            merged = mergeDiscoveredDevice(merged, d);
          }

          dispatch({
            type: "set_snapshot",
            snapshot: {
              mode,
              status,
              devices: merged,
              error,
              scan: scanShape,
              ipScan: ipScan
                ? { expanded: false, defaultCidr, candidates }
                : undefined,
            },
          });
        })();
      }, 1000);
    })();
  }, [open]);

  useEffect(() => {
    if (!open || method !== "local_usb") {
      return;
    }
    let cancelled = false;

    const loadLocalUsbPorts = async () => {
      setAddError(null);
      setUsbStatus("Looking for Local USB ports...");
      try {
        const agent = agentRef.current ?? (await tryBootstrapCompanionBridge());
        agentRef.current = agent;
        if (cancelled || methodRef.current !== "local_usb") {
          return;
        }
        if (!agent) {
          setLocalUsbPorts([]);
          setSelectedLocalUsbPort("");
          setAddError("Local companion is not running.");
          return;
        }
        const ports = filterEsp32SerialPorts(
          await listLocalUsbSerialPorts(agent),
        );
        if (cancelled || methodRef.current !== "local_usb") {
          return;
        }
        setLocalUsbPorts(ports);
        setSelectedLocalUsbPort((current) =>
          ports.some((port) => port.path === current) ? current : "",
        );
        if (ports.length === 0) {
          setAddError("No ESP32 USB serial ports found.");
          setUsbStatus(null);
          return;
        }
        setUsbStatus(
          ports.length === 1
            ? "Local USB device ready. Click it to connect."
            : "Choose a Local USB device to connect.",
        );
      } catch (err) {
        if (!cancelled && methodRef.current === "local_usb") {
          setLocalUsbPorts([]);
          setSelectedLocalUsbPort("");
          setAddError(
            err instanceof Error ? err.message : "Local USB port list failed.",
          );
        }
      }
    };

    void loadLocalUsbPorts();
    return () => {
      cancelled = true;
    };
  }, [open, method]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const ipScan = snapshot.ipScan;
    if (!ipScan || ipScan.expanded) {
      return;
    }
    if (snapshot.mode !== "service" || snapshot.status !== "scanning") {
      return;
    }
    if (!ipScan.autoExpandAfterMs) {
      return;
    }

    const expectedCount = devicesCountRef.current;
    const timer = window.setTimeout(() => {
      if (devicesCountRef.current !== expectedCount) {
        return;
      }
      if (ipScanExpandedRef.current) {
        return;
      }
      dispatch({
        type: "toggle_ip_scan",
        expanded: true,
        expandedBy: "auto",
      });
      dispatch({
        type: "set_error",
        error:
          "No devices found yet — try IP scan (advanced) with a CIDR range.",
      });
    }, ipScan.autoExpandAfterMs);
    return () => window.clearTimeout(timer);
  }, [open, snapshot.ipScan, snapshot.mode, snapshot.status]);

  const addDiscoveredDevice = async (device: DiscoveredDevice) => {
    if (!device.baseUrl) {
      setAddError("Discovered hub did not include a network URL.");
      return;
    }
    const input: AddDeviceInput = {
      name:
        device.hostname ?? device.fqdn ?? device.device_id ?? "IsoHub USB Hub",
      baseUrl: device.baseUrl,
      id: device.device_id,
    };
    const saved = await onCreate(input);
    if (!saved.ok) {
      setAddError(
        saved.errors.baseUrl ??
          saved.errors.id ??
          saved.errors.name ??
          "Could not add this hub.",
      );
      return;
    }
    setAddError(null);
    onClose();
  };

  const resolveReachableUsbBaseUrl = async (
    device: UsbDeviceInfo,
    id: string,
    hostname: string,
    run?: { id: number; method: AddDeviceMethod },
  ): Promise<string> => {
    const fallbackBaseUrl = `http://${device.fqdn?.trim() || `${hostname}.local`}`;
    const ipv4 = device.wifi?.ipv4?.trim();
    if (!ipv4) {
      setUsbStep("USB info did not report a Wi-Fi IPv4 address.", "warning");
      return fallbackBaseUrl;
    }

    const wifiBaseUrl = `http://${ipv4}`;
    setUsbStep(`Checking Wi-Fi reachability at ${wifiBaseUrl}...`);
    const res = await getDeviceInfo(wifiBaseUrl);
    if (run && !isActiveUsbRun(run.id, run.method)) {
      return fallbackBaseUrl;
    }
    if (!res.ok) {
      setUsbStep(
        `Wi-Fi reported ${ipv4}, but HTTP is not reachable yet: ${res.error.message}`,
        "warning",
      );
      return fallbackBaseUrl;
    }
    if (!usbInfoMatchesHttpInfo(device, id, res.value)) {
      setUsbStep(
        "Wi-Fi HTTP responded, but identity did not match the USB device.",
        "warning",
      );
      return fallbackBaseUrl;
    }

    setUsbStep("Wi-Fi HTTP link verified and will be saved.", "success");
    announceNetworkDeviceLink({ deviceId: id, baseUrl: wifiBaseUrl });
    return wifiBaseUrl;
  };

  const addUsbDevice = async (
    envelope: unknown,
    fallback?: {
      serialNumber?: string | null;
      portPath?: string;
      webSerialTransport?: WebSerialJsonlTransport;
    },
    run?: { id: number; method: AddDeviceMethod },
  ): Promise<boolean> => {
    if (run && !isActiveUsbRun(run.id, run.method)) {
      return false;
    }

    const parsed = parseUsbInfoEnvelope(envelope);
    if (!parsed.ok) {
      setAddError(parsed.error);
      return false;
    }

    const device = parsed.device;
    const id =
      normalizeDeviceId(device.device_id) ??
      shortIdFromMac(device.mac) ??
      shortIdFromMac(fallback?.serialNumber ?? "");

    if (!id) {
      setAddError("Connected device did not report a stable device ID.");
      return false;
    }

    const hostname = device.hostname?.trim() || `isohub-${id}`;
    const baseUrl = await resolveReachableUsbBaseUrl(device, id, hostname, run);
    if (run && !isActiveUsbRun(run.id, run.method)) {
      return false;
    }

    setUsbStep("Saving hub profile...");
    const existingName = existingDeviceNamesById?.[id]?.trim();
    const input = {
      id,
      name: existingName || hostname,
      baseUrl,
    };
    const saved = ids.includes(id)
      ? await onUpsert(input)
      : await onCreate(input);
    if (run && !isActiveUsbRun(run.id, run.method)) {
      return false;
    }
    if (!saved.ok) {
      if (saved.errors.id === "ID already exists") {
        if (fallback?.portPath) {
          announceLocalUsbDeviceLink({
            deviceId: id,
            portPath: fallback.portPath,
          });
        }
        if (fallback?.webSerialTransport) {
          announceWebSerialDeviceLink({
            deviceId: id,
            transport: fallback.webSerialTransport,
          });
        }
        const updated = await onUpsert(input);
        if (updated.ok) {
          setUsbStep(
            "Existing hub updated with the latest connection link.",
            "success",
          );
          setAddError(null);
          onClose();
          navigate(`/devices/${id}`);
          return true;
        }
      }
      setAddError(
        saved.errors.id ??
          saved.errors.baseUrl ??
          saved.errors.name ??
          "Could not add this hub.",
      );
      return false;
    }
    setUsbStep("Hub saved.", "success");
    if (fallback?.portPath) {
      announceLocalUsbDeviceLink({ deviceId: id, portPath: fallback.portPath });
    }
    if (fallback?.webSerialTransport) {
      announceWebSerialDeviceLink({
        deviceId: id,
        transport: fallback.webSerialTransport,
      });
    }
    setAddError(null);
    onClose();
    navigate(`/devices/${id}`);
    return true;
  };

  const connectByLocalUsb = async (portPath?: string) => {
    const runId = startUsbRun("local_usb");
    setUsbBusy(true);
    setAddError(null);
    setUsbLog([]);
    setUsbStep("Preparing Local USB connection...");
    try {
      const agent = agentRef.current ?? (await tryBootstrapCompanionBridge());
      agentRef.current = agent;
      if (!isActiveUsbRun(runId, "local_usb")) {
        return;
      }
      if (!agent) {
        setAddError("Local companion is not running.");
        return;
      }
      const ports =
        localUsbPorts.length > 0
          ? localUsbPorts
          : filterEsp32SerialPorts(await listLocalUsbSerialPorts(agent));
      setLocalUsbPorts(ports);
      if (!isActiveUsbRun(runId, "local_usb")) {
        return;
      }
      if (ports.length === 0) {
        setAddError("No ESP32 USB serial ports found.");
        return;
      }

      const selectedPortPath = portPath ?? selectedLocalUsbPort;
      if (selectedPortPath) {
        setSelectedLocalUsbPort(selectedPortPath);
        const port = ports.find((p) => p.path === selectedPortPath);
        if (!port) {
          setUsbStep("Choose the IsoHub ESP32 USB device to connect.");
          return;
        }
        setUsbStep(`Opening Local USB port ${port.path}...`);
        const response = await readLocalUsbInfo(agent, port, appendUsbLog);
        await addUsbDevice(
          response,
          { serialNumber: port.serialNumber, portPath: port.path },
          { id: runId, method: "local_usb" },
        );
        return;
      }

      setUsbStep("Identifying IsoHub USB hub...");
      for (const port of ports) {
        try {
          setUsbStep(`Trying Local USB port ${port.path}...`);
          const response = await readLocalUsbInfo(agent, port, appendUsbLog);
          if (!isActiveUsbRun(runId, "local_usb")) {
            return;
          }
          const parsed = parseUsbInfoEnvelope(response);
          if (!parsed.ok || !isIsoHubDeviceInfo(parsed.device)) {
            continue;
          }
          setSelectedLocalUsbPort(port.path);
          await addUsbDevice(
            response,
            { serialNumber: port.serialNumber, portPath: port.path },
            { id: runId, method: "local_usb" },
          );
          return;
        } catch {
          // Keep probing other ESP32 serial ports.
        }
      }

      if (ports.length === 1) {
        setAddError("The ESP32 USB port did not respond as IsoHub.");
        appendUsbLog(
          "Local USB info request did not identify IsoHub.",
          "error",
        );
        return;
      }
      setUsbStep("Choose the IsoHub ESP32 USB device to connect.");
    } catch (err) {
      if (isActiveUsbRun(runId, "local_usb")) {
        const message =
          err instanceof Error ? err.message : "Local USB failed.";
        appendUsbLog(message, "error");
        setAddError(message);
      }
    } finally {
      if (isActiveUsbRun(runId, "local_usb")) {
        setUsbBusy(false);
      }
    }
  };

  const connectByWebSerial = async () => {
    const runId = startUsbRun("web_serial");
    setUsbBusy(true);
    setAddError(null);
    setUsbLog([]);
    setUsbStep("Requesting browser serial access...");
    const transport = new WebSerialJsonlTransport();
    let handedOff = false;
    try {
      await transport.connectWithPicker();
      if (!isActiveUsbRun(runId, "web_serial")) {
        return;
      }
      setUsbStep("Browser serial port opened. Reading connected hub...");
      const response = await readWebSerialInfo(transport, appendUsbLog);
      handedOff = await addUsbDevice(
        response,
        { webSerialTransport: transport },
        {
          id: runId,
          method: "web_serial",
        },
      );
    } catch (err) {
      if (isActiveUsbRun(runId, "web_serial")) {
        const message =
          err instanceof Error ? err.message : "Web Serial failed.";
        appendUsbLog(message, "error");
        setAddError(message);
      }
    } finally {
      if (!handedOff) {
        await transport.disconnect().catch(() => undefined);
      }
      if (isActiveUsbRun(runId, "web_serial")) {
        setUsbBusy(false);
      }
    }
  };

  const selectMethod = (nextMethod: AddDeviceMethod) => {
    if (nextMethod === methodRef.current) {
      return;
    }
    usbRunIdRef.current += 1;
    methodRef.current = nextMethod;
    setMethod(nextMethod);
    setAddError(null);
    setUsbStatus(null);
    setUsbLog([]);
    setUsbBusy(false);
  };

  const startUsbRun = (runMethod: AddDeviceMethod) => {
    const runId = usbRunIdRef.current + 1;
    usbRunIdRef.current = runId;
    methodRef.current = runMethod;
    return runId;
  };

  const isActiveUsbRun = (runId: number, runMethod: AddDeviceMethod) =>
    openRef.current &&
    usbRunIdRef.current === runId &&
    methodRef.current === runMethod;

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const methodOptions: Array<{
    id: AddDeviceMethod;
    title: string;
    description: string;
  }> = [
    {
      id: "wifi",
      title: "Wi-Fi / LAN",
      description: "Discover or add a hub already reachable on the network.",
    },
    {
      id: "web_serial",
      title: "Web Serial",
      description: "Use the browser USB serial path to identify and add a hub.",
    },
    {
      id: "local_usb",
      title: "Local USB",
      description: "Use the local companion for USB identification.",
    },
  ];

  return (
    <dialog
      ref={dialogRef}
      className="modal"
      aria-label="Add device"
      data-testid="add-device-dialog"
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onClick={(e) => {
        if (e.target === dialogRef.current) {
          onClose();
        }
      }}
      onKeyDown={(e) => {
        if (e.target !== dialogRef.current) {
          return;
        }
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClose();
        }
      }}
    >
      <div className="modal-box iso-modal flex max-h-[calc(100vh-32px)] w-[1040px] max-w-[calc(100vw-32px)] flex-col overflow-hidden rounded-[22px] border border-[var(--border)] bg-[var(--panel)] px-8 pb-7 pt-6">
        <div className="text-[24px] font-bold">Add device</div>
        <div className="mt-2 text-[14px] font-medium text-[var(--muted)]">
          Store locally; used for Dashboard and device pages.
        </div>

        <div
          className="mt-6 grid grid-cols-1 gap-3 min-[760px]:grid-cols-3"
          role="tablist"
          aria-label="Connection method"
        >
          {methodOptions.map((option) => {
            const selected = method === option.id;
            return (
              <button
                key={option.id}
                className={[
                  "min-h-[86px] rounded-[14px] border px-4 py-3 text-left transition-colors",
                  selected
                    ? "border-[var(--primary)] bg-[var(--panel)] shadow-[inset_0_0_0_1px_var(--primary)]"
                    : "border-[var(--border)] bg-[var(--panel-2)] hover:border-[var(--primary)]",
                ].join(" ")}
                type="button"
                role="tab"
                aria-selected={selected}
                onClick={() => selectMethod(option.id)}
              >
                <div className="text-[14px] font-bold text-[var(--text)]">
                  {option.title}
                </div>
                <div className="mt-2 text-[12px] font-semibold leading-5 text-[var(--muted)]">
                  {option.description}
                </div>
              </button>
            );
          })}
        </div>

        <div className="mt-6 flex min-h-0 flex-1 flex-col gap-6">
          <div className="min-h-0 min-w-0 flex-1">
            {method === "wifi" ? (
              <>
                <DeviceDiscoveryPanel
                  key={discoveryPanelKey}
                  snapshot={snapshot}
                  existingDeviceIds={ids}
                  existingDeviceBaseUrls={baseUrls}
                  onRefresh={() => {
                    scanRunIdRef.current += 1;
                    const agent = agentRef.current;
                    if (agent) {
                      dispatch({ type: "reset", status: "scanning" });
                      void agentFetch(agent, "/api/v1/discovery/refresh", {
                        method: "POST",
                        body: JSON.stringify({}),
                      });
                    } else {
                      dispatch({ type: "reset", status: "unavailable" });
                    }
                  }}
                  onToggleIpScan={(expanded) =>
                    dispatch({
                      type: "toggle_ip_scan",
                      expanded,
                      expandedBy: "user",
                    })
                  }
                  onStartScan={(cidr) => {
                    const parsed = parseCidr(cidr);
                    if (!parsed.ok) {
                      dispatch({ type: "set_error", error: parsed.error });
                      return;
                    }

                    const agent = agentRef.current;
                    if (agent) {
                      dispatch({
                        type: "start_scan",
                        cidr: parsed.cidr,
                        total: parsed.hosts.length,
                      });
                      void agentFetch(agent, "/api/v1/discovery/ip-scan", {
                        method: "POST",
                        body: JSON.stringify({ cidr: parsed.cidr }),
                      });
                      return;
                    }

                    scanRunIdRef.current += 1;
                    const runId = scanRunIdRef.current;

                    dispatch({
                      type: "start_scan",
                      cidr: parsed.cidr,
                      total: parsed.hosts.length,
                    });

                    const concurrency = 12;
                    let nextIndex = 0;
                    let done = 0;
                    let preflightBlocked = false;

                    const worker = async () => {
                      for (;;) {
                        if (scanRunIdRef.current !== runId) {
                          return;
                        }
                        const idx = nextIndex;
                        nextIndex += 1;
                        if (idx >= parsed.hosts.length) {
                          return;
                        }

                        const ip = parsed.hosts[idx];
                        const baseUrlByIp = `http://${ip}`;
                        const res = await getDeviceInfo(baseUrlByIp);
                        if (scanRunIdRef.current !== runId) {
                          return;
                        }
                        done += 1;
                        dispatch({ type: "scan_progress", done });

                        if (!res.ok) {
                          if (res.error.kind === "preflight_blocked") {
                            preflightBlocked = true;
                          }
                          continue;
                        }

                        const nowIso = new Date().toISOString();
                        const device = parseDiscoveredDeviceFromApiInfo(
                          baseUrlByIp,
                          res.value as unknown,
                          ip,
                          nowIso,
                        );
                        if (!device) {
                          continue;
                        }
                        dispatch({ type: "scan_device", device });
                      }
                    };

                    void (async () => {
                      await Promise.all(
                        Array.from({ length: concurrency }, () => worker()),
                      );
                      if (scanRunIdRef.current !== runId) {
                        return;
                      }
                      if (preflightBlocked) {
                        dispatch({
                          type: "set_error",
                          error:
                            "Local network access blocked (PNA/CORS preflight). Try allowing private network access, or connect by USB first.",
                        });
                      }
                      dispatch({ type: "scan_done" });
                    })();
                  }}
                  onCancelScan={() => {
                    scanRunIdRef.current += 1;
                    dispatch({ type: "scan_cancelled" });
                    const agent = agentRef.current;
                    if (agent) {
                      void agentFetch(agent, "/api/v1/discovery/cancel", {
                        method: "POST",
                        body: JSON.stringify({}),
                      });
                    }
                  }}
                  onSelect={(device: DiscoveredDevice) => {
                    void addDiscoveredDevice(device);
                  }}
                />
                {addError ? <InlineAddError message={addError} /> : null}
              </>
            ) : (
              <div className="flex min-h-[360px] flex-col justify-between rounded-[16px] border border-[var(--border)] bg-[var(--panel-2)] p-5">
                <div>
                  <div className="text-[16px] font-bold">
                    {method === "web_serial"
                      ? "Add by Web Serial"
                      : "Add by Local USB"}
                  </div>
                  <div className="mt-3 text-[13px] font-semibold leading-6 text-[var(--muted)]">
                    {method === "web_serial"
                      ? "Select the hub in the browser serial picker. The app reads device info over USB and adds it here."
                      : "Use the local companion service to read the connected hub over USB and add it here."}
                  </div>
                  {method === "web_serial" && !isWebSerialSupported() ? (
                    <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--panel)] px-4 py-3 text-[12px] font-semibold text-[var(--warning)]">
                      This browser does not expose Web Serial. Use Chrome/Edge
                      or Local USB.
                    </div>
                  ) : null}
                  {method === "local_usb" && localUsbPorts.length > 0 ? (
                    <div className="mt-5">
                      <div className="text-[12px] font-bold text-[var(--muted)]">
                        Local USB devices
                      </div>
                      <div className="mt-2 grid gap-2">
                        {localUsbPorts.map((port) => {
                          const active = selectedLocalUsbPort === port.path;
                          return (
                            <button
                              key={port.path}
                              className={[
                                "flex min-h-[58px] w-full items-center justify-between gap-4 rounded-[12px] border px-4 py-3 text-left",
                                active
                                  ? "border-[var(--primary)] bg-[var(--panel)]"
                                  : "border-[var(--border)] bg-[var(--panel)]",
                                usbBusy
                                  ? "cursor-not-allowed opacity-70"
                                  : "hover:border-[var(--primary)]",
                              ].join(" ")}
                              type="button"
                              disabled={usbBusy}
                              onClick={() => void connectByLocalUsb(port.path)}
                            >
                              <span className="min-w-0">
                                <span className="block truncate text-[13px] font-bold text-[var(--text)]">
                                  {port.label}
                                </span>
                                <span className="mt-1 block truncate font-mono text-[12px] font-semibold text-[var(--muted)]">
                                  {port.path}
                                </span>
                              </span>
                              <span className="shrink-0 text-[12px] font-bold text-[var(--muted)]">
                                {usbBusy && active
                                  ? "Connecting..."
                                  : "Connect"}
                              </span>
                            </button>
                          );
                        })}
                      </div>
                    </div>
                  ) : null}
                  {usbStatus ? (
                    <div className="mt-4 text-[12px] font-semibold text-[var(--muted)]">
                      {usbStatus}
                    </div>
                  ) : null}
                  {usbLog.length > 0 ? (
                    <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--panel)] px-4 py-3">
                      <div className="text-[12px] font-bold text-[var(--muted)]">
                        Connection log
                      </div>
                      <div className="mt-2 grid gap-1.5">
                        {usbLog.map((entry) => (
                          <div
                            key={entry.id}
                            className={[
                              "flex min-w-0 items-start gap-2 text-[12px] font-semibold leading-5",
                              entry.tone === "success"
                                ? "text-[var(--badge-success-text)]"
                                : entry.tone === "warning"
                                  ? "text-[var(--warning)]"
                                  : entry.tone === "error"
                                    ? "text-[var(--error)]"
                                    : "text-[var(--muted)]",
                            ].join(" ")}
                          >
                            <span className="mt-[7px] h-1.5 w-1.5 shrink-0 rounded-full bg-current" />
                            <span className="min-w-0 break-words">
                              {entry.message}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  ) : null}
                  {addError ? <InlineAddError message={addError} /> : null}
                </div>

                {method === "web_serial" ? (
                  <div className="mt-8 grid gap-3">
                    <button
                      className="btn h-12 justify-center"
                      type="button"
                      disabled={usbBusy || !isWebSerialSupported()}
                      onClick={() => void connectByWebSerial()}
                    >
                      {usbBusy ? "Connecting..." : "Connect and add"}
                    </button>
                  </div>
                ) : null}
              </div>
            )}
          </div>
        </div>

        <div className="mt-6 flex items-center justify-end">
          <button className="btn" type="button" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </dialog>
  );
}
