import { type ReactNode, useLayoutEffect, useRef, useState } from "react";

import type {
  ConnectionState,
  DeviceTransport,
} from "../../app/device-runtime";
import type { StoredDevice } from "../../domain/devices";

export type DeviceTransportBadge = {
  transport: DeviceTransport;
  state: "primary" | "connected" | "history";
};

function badgeStyles(state: ConnectionState): {
  bg: string;
  text: string;
  width: string;
} {
  if (state === "online") {
    return {
      bg: "bg-[var(--badge-success-bg)]",
      text: "text-[var(--badge-success-text)]",
      width: "w-[96px]",
    };
  }
  if (state === "offline") {
    return {
      bg: "bg-[var(--badge-error-bg)]",
      text: "text-[var(--badge-error-text)]",
      width: "w-[96px]",
    };
  }
  return {
    bg: "bg-[var(--badge-warning-bg)]",
    text: "text-[var(--badge-warning-text)]",
    width: "w-[96px]",
  };
}

export type DeviceCardProps = {
  device: StoredDevice;
  selected?: boolean;
  status: ConnectionState;
  transportBadges: DeviceTransportBadge[];
  unselectedFill: "panel" | "panel-2";
  onSelect: (deviceId: string) => void;
};

function transportLabel(transport: DeviceTransport): string {
  if (transport === "http") {
    return "Wi-Fi";
  }
  if (transport === "web_serial") {
    return "Serial";
  }
  if (transport === "local_usb") {
    return "USB";
  }
  return "Unknown";
}

function transportFullLabel(transport: DeviceTransport): string {
  if (transport === "http") {
    return "Wi-Fi / LAN";
  }
  if (transport === "web_serial") {
    return "Web Serial";
  }
  if (transport === "local_usb") {
    return "Local USB";
  }
  return "Unknown";
}

function transportBadgeStyles(state: DeviceTransportBadge["state"]): {
  bg: string;
  text: string;
  border: string;
  opacity: string;
} {
  if (state === "primary") {
    return {
      bg: "bg-[var(--primary)]",
      text: "text-[var(--primary-text)]",
      border: "border-transparent",
      opacity: "",
    };
  }
  if (state === "connected") {
    return {
      bg: "bg-[var(--panel-2)]",
      text: "text-[var(--text)]",
      border: "border-[var(--border)]",
      opacity: "",
    };
  }
  return {
    bg: "bg-[var(--panel)]",
    text: "text-[var(--muted)]",
    border: "border-[var(--border)]",
    opacity: "opacity-70",
  };
}

function transportIcon(transport: DeviceTransport): ReactNode {
  const className = "h-3.5 w-3.5 shrink-0";
  if (transport === "http") {
    return (
      <svg aria-hidden="true" className={className} viewBox="0 0 16 16">
        <circle
          cx="8"
          cy="8"
          r="5.75"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.25"
        />
        <path
          d="M2.75 8h10.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.25"
          strokeLinecap="round"
        />
        <path
          d="M8 2.25c1.7 1.4 2.8 3.5 2.8 5.75S9.7 12.35 8 13.75"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.25"
          strokeLinecap="round"
        />
        <path
          d="M8 2.25c-1.7 1.4-2.8 3.5-2.8 5.75S6.3 12.35 8 13.75"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.25"
          strokeLinecap="round"
        />
      </svg>
    );
  }
  if (transport === "web_serial") {
    return (
      <svg aria-hidden="true" className={className} viewBox="0 0 16 16">
        <rect
          x="1.75"
          y="4.75"
          width="4"
          height="6.5"
          rx="1.1"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.25"
        />
        <rect
          x="10.25"
          y="4.75"
          width="4"
          height="6.5"
          rx="1.1"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.25"
        />
        <path
          d="M5.75 8h4.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.25"
          strokeLinecap="round"
        />
      </svg>
    );
  }
  return (
    <svg aria-hidden="true" className={className} viewBox="0 0 16 16">
      <path
        d="M7 1.75v7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.25"
        strokeLinecap="round"
      />
      <path
        d="M9 1.75v7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.25"
        strokeLinecap="round"
      />
      <path
        d="M3.75 8.75h8.5v2a4.25 4.25 0 0 1-8.5 0z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.25"
        strokeLinejoin="round"
      />
      <path
        d="M8 11v3"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.25"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function DeviceCard({
  device,
  selected,
  status,
  transportBadges,
  unselectedFill,
  onSelect,
}: DeviceCardProps) {
  const fill =
    selected || unselectedFill === "panel"
      ? "bg-[var(--panel)]"
      : "bg-[var(--panel-2)]";
  const badge = badgeStyles(status);
  const transportRowRef = useRef<HTMLDivElement | null>(null);
  const transportMeasureRef = useRef<HTMLDivElement | null>(null);
  const [iconOnly, setIconOnly] = useState(false);

  useLayoutEffect(() => {
    const row = transportRowRef.current;
    const measureRow = transportMeasureRef.current;
    if (!row || !measureRow) {
      return;
    }

    const measure = () => {
      window.requestAnimationFrame(() => {
        const current = transportRowRef.current;
        const currentMeasure = transportMeasureRef.current;
        if (!current || !currentMeasure) {
          return;
        }
        setIconOnly(currentMeasure.scrollWidth > current.clientWidth + 1);
      });
    };

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(row);
    return () => observer.disconnect();
  }, []);

  return (
    <button
      data-testid={`device-card-${device.id}`}
      className={[
        "w-full rounded-[14px] border border-[var(--border)]",
        "px-5 py-4 text-left",
        fill,
        selected ? "iso-card" : "",
      ].join(" ")}
      type="button"
      onClick={() => onSelect(device.id)}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="truncate text-[14px] font-medium">{device.name}</div>
          <div className="relative mt-3 min-w-0">
            <div
              ref={transportMeasureRef}
              aria-hidden="true"
              className="pointer-events-none invisible absolute inset-x-0 top-0 flex flex-nowrap gap-1.5"
            >
              {transportBadges.map((badge) => {
                const styles = transportBadgeStyles(badge.state);
                return (
                  <div
                    key={`${device.id}-${badge.transport}-measure`}
                    className={[
                      "inline-flex h-7 shrink-0 items-center gap-1.5 rounded-full border px-2",
                      styles.bg,
                      styles.text,
                      styles.border,
                      styles.opacity,
                      "text-[12px] font-semibold",
                    ].join(" ")}
                  >
                    {transportIcon(badge.transport)}
                    <span className="truncate">
                      {transportLabel(badge.transport)}
                    </span>
                  </div>
                );
              })}
            </div>
            <div
              ref={transportRowRef}
              className="flex min-w-0 flex-nowrap gap-1.5 overflow-hidden"
            >
              {transportBadges.map((badge) => {
                const styles = transportBadgeStyles(badge.state);
                return (
                  <div
                    key={`${device.id}-${badge.transport}`}
                    title={transportFullLabel(badge.transport)}
                    className={[
                      "inline-flex h-7 shrink-0 items-center rounded-full border",
                      iconOnly ? "w-7 justify-center px-0" : "gap-1.5 px-2",
                      styles.bg,
                      styles.text,
                      styles.border,
                      styles.opacity,
                      "text-[12px] font-semibold",
                    ].join(" ")}
                  >
                    {transportIcon(badge.transport)}
                    {iconOnly ? null : (
                      <span className="truncate">
                        {transportLabel(badge.transport)}
                      </span>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
        <div
          className={[
            "flex h-[22px] items-center justify-center rounded-full",
            badge.width,
            badge.bg,
            badge.text,
            "text-[12px] font-semibold",
          ].join(" ")}
        >
          {status}
        </div>
      </div>
    </button>
  );
}
