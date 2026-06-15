import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  deleteStoredDevice,
  exportStorage,
  fetchStoredDevices,
  migrateFromLocalStorage,
  upsertStoredDevice,
} from "../domain/companionStorage";
import type {
  AddDeviceInput,
  AddDeviceValidationResult,
  StoredDevice,
} from "../domain/devices";
import {
  loadStoredDevices,
  normalizeBaseUrl,
  saveStoredDevices,
  validateAddDeviceInput,
} from "../domain/devices";
import { forgetLocalUsbDeviceLink } from "../domain/localUsbLinks";
import { forgetWebSerialDeviceTransport } from "../domain/webSerialLinks";
import { useToast } from "../ui/toast/ToastProvider";
import { useCompanionBridge } from "./companion-bridge-ui";
import { readMigrationPayload } from "./storage-migration";

type DevicesContextValue = {
  devices: StoredDevice[];
  addDevice: (input: AddDeviceInput) => Promise<AddDeviceValidationResult>;
  upsertDevice: (input: AddDeviceInput) => Promise<AddDeviceValidationResult>;
  persistDevice: (device: StoredDevice) => Promise<StoredDevice | null>;
  removeDevice: (deviceId: string) => Promise<void>;
  refreshDevices: () => Promise<void>;
  getDevice: (deviceId: string) => StoredDevice | undefined;
};

const DevicesContext = createContext<DevicesContextValue | null>(null);

export function DevicesProvider({
  children,
  initialDevices,
}: {
  children: React.ReactNode;
  initialDevices?: StoredDevice[];
}) {
  const { agent, status } = useCompanionBridge();
  const { pushToast } = useToast();
  const warnedRef = useRef(false);
  const [devices, setDevices] = useState<StoredDevice[]>(() =>
    initialDevices ? initialDevices : loadStoredDevices(),
  );
  const [ready, setReady] = useState(false);

  const refreshDevices = useCallback(async () => {
    if (agent) {
      const res = await fetchStoredDevices(agent);
      if (!res.ok) {
        pushToast({
          variant: "error",
          message: `Local companion storage unavailable: ${res.error.message}`,
        });
        return;
      }
      setDevices(res.value);
      return;
    }
    if (!initialDevices) {
      setDevices(loadStoredDevices());
    }
  }, [agent, pushToast, initialDevices]);

  useEffect(() => {
    if (!ready) {
      return;
    }
    if (agent) {
      return;
    }
    saveStoredDevices(devices);
  }, [devices, agent, ready]);

  useEffect(() => {
    if (status !== "ready") {
      return;
    }
    let cancelled = false;
    void (async () => {
      await refreshDevices();
      if (cancelled) {
        return;
      }
      setReady(true);
    })();
    return () => {
      cancelled = true;
    };
  }, [status, refreshDevices]);

  useEffect(() => {
    if (status !== "ready" || !agent) {
      return;
    }
    void (async () => {
      const payload = readMigrationPayload();
      if (!payload) {
        return;
      }
      const res = await migrateFromLocalStorage(agent, payload);
      if (!res.ok) {
        return;
      }
      if (res.value.migrated) {
        pushToast({
          variant: "success",
          message: "Imported devices/settings from browser storage.",
        });
        window.dispatchEvent(new CustomEvent("isohub-storage-migrated"));
        const refreshed = await fetchStoredDevices(agent);
        if (refreshed.ok) {
          setDevices(refreshed.value);
        }
      }
    })();
  }, [agent, status, pushToast]);

  useEffect(() => {
    if (status !== "ready" || !agent || warnedRef.current) {
      return;
    }
    void (async () => {
      const res = await exportStorage(agent);
      if (!res.ok) {
        return;
      }
      const meta = res.value.meta;
      if (meta?.last_corrupt_at) {
        warnedRef.current = true;
        pushToast({
          variant: "warning",
          message: "Local storage was reset after a corruption.",
        });
      }
    })();
  }, [agent, status, pushToast]);

  const value = useMemo<DevicesContextValue>(() => {
    const existingIds = new Set(devices.map((d) => d.id));
    const existingBaseUrls = new Set(devices.map((d) => d.baseUrl));

    const persistDevice = async (
      device: StoredDevice,
    ): Promise<AddDeviceValidationResult> => {
      if (!agent) {
        setDevices((prev) => {
          const next = prev.filter(
            (d) => d.id !== device.id && d.baseUrl !== device.baseUrl,
          );
          return [...next, device];
        });
        return { ok: true, device };
      }
      const res = await upsertStoredDevice(agent, device);
      if (!res.ok) {
        if (res.error.code === "conflict") {
          return {
            ok: false,
            errors: { baseUrl: res.error.message },
          };
        }
        pushToast({
          variant: "error",
          message: `Local companion storage error: ${res.error.message}`,
        });
        return {
          ok: false,
          errors: { baseUrl: "Local companion storage unavailable" },
        };
      }
      setDevices((prev) => {
        const next = prev.filter(
          (d) => d.id !== res.value.id && d.baseUrl !== res.value.baseUrl,
        );
        return [...next, res.value];
      });
      return { ok: true, device: res.value };
    };

    const persistExistingDevice = async (
      device: StoredDevice,
    ): Promise<StoredDevice | null> => {
      const res = await persistDevice(device);
      return res.ok ? res.device : null;
    };

    return {
      devices,
      addDevice: async (input) => {
        const result = validateAddDeviceInput(
          input,
          existingIds,
          existingBaseUrls,
        );
        if (!result.ok) {
          return result;
        }
        return persistDevice(result.device);
      },
      upsertDevice: async (input) => {
        const name = input.name.trim();
        const id = input.id?.trim();
        const baseUrl = normalizeBaseUrl(input.baseUrl);
        if (!name || !id || !baseUrl.ok) {
          return {
            ok: false,
            errors: {
              name: name ? undefined : "Name is required",
              id: id ? undefined : "ID is required",
              baseUrl: baseUrl.ok ? undefined : baseUrl.error,
            },
          };
        }
        if (devices.some((d) => d.id !== id && d.baseUrl === baseUrl.baseUrl)) {
          return { ok: false, errors: { baseUrl: "Base URL already exists" } };
        }
        return persistDevice({
          id,
          name,
          baseUrl: baseUrl.baseUrl,
          transports: input.transports,
        });
      },
      persistDevice: persistExistingDevice,
      removeDevice: async (deviceId) => {
        if (agent) {
          const res = await deleteStoredDevice(agent, deviceId);
          if (!res.ok) {
            pushToast({
              variant: "error",
              message: `Local companion storage error: ${res.error.message}`,
            });
            throw new Error(res.error.message);
          }
        }
        forgetLocalUsbDeviceLink(deviceId);
        forgetWebSerialDeviceTransport(deviceId);
        setDevices((prev) => prev.filter((d) => d.id !== deviceId));
      },
      refreshDevices,
      getDevice: (deviceId) => devices.find((d) => d.id === deviceId),
    };
  }, [devices, agent, pushToast, refreshDevices]);

  return (
    <DevicesContext.Provider value={value}>{children}</DevicesContext.Provider>
  );
}

export function useDevices(): DevicesContextValue {
  const ctx = useContext(DevicesContext);
  if (!ctx) {
    throw new Error("useDevices must be used within <DevicesProvider>");
  }
  return ctx;
}
