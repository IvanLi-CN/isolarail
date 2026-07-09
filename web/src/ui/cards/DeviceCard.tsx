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
  tone: string;
  width: string;
} {
  if (state === "online") {
    return {
      tone: "iso-chip--success",
      width: "w-[96px]",
    };
  }
  if (state === "offline") {
    return {
      tone: "iso-chip--error",
      width: "w-[96px]",
    };
  }
  return {
    tone: "iso-chip--warning",
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
  tone: string;
  opacity: string;
} {
  if (state === "primary") {
    return {
      tone: "iso-chip--signal",
      opacity: "",
    };
  }
  if (state === "connected") {
    return {
      tone: "iso-chip--neutral",
      opacity: "",
    };
  }
  return {
    tone: "",
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
  const fill = selected
    ? ""
    : unselectedFill === "panel"
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
        "w-full rounded-[16px] border px-4 py-4 text-left",
        fill,
        selected
          ? "border-[color-mix(in_srgb,var(--primary)_42%,var(--border))] bg-[color-mix(in_srgb,var(--badge-signal-bg)_55%,var(--panel))]"
          : "border-[var(--border)]",
      ].join(" ")}
      type="button"
      onClick={() => onSelect(device.id)}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="truncate text-[16px] font-bold leading-5">
            {device.name}
          </div>
          <div className="mt-2 truncate font-mono text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--muted)]">
            {device.baseUrl.replace(/^https?:\/\//, "")}
          </div>
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
                      "iso-chip h-7 shrink-0 gap-1.5 px-2",
                      styles.tone,
                      styles.opacity,
                      "text-[11px]",
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
                      "iso-chip h-7 shrink-0 items-center",
                      iconOnly ? "w-7 justify-center px-0" : "gap-1.5 px-2",
                      styles.tone,
                      styles.opacity,
                      "text-[11px]",
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
            "iso-chip h-7 items-center justify-center",
            badge.width,
            badge.tone,
            "text-[11px]",
          ].join(" ")}
        >
          {status}
        </div>
      </div>
    </button>
  );
}
