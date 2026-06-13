import { useEffect, useRef, useState } from "react";
import type { PortCardProps } from "./types";

function statusBadgeStyles(status: string): { bg: string; text: string } {
  if (status === "ok") {
    return {
      bg: "bg-[var(--badge-success-bg)]",
      text: "text-[var(--badge-success-text)]",
    };
  }
  if (status === "error") {
    return {
      bg: "bg-[var(--badge-error-bg)]",
      text: "text-[var(--badge-error-text)]",
    };
  }
  return {
    bg: "bg-[var(--badge-warning-bg)]",
    text: "text-[var(--badge-warning-text)]",
  };
}

function formatValue(value: number | null, unit: "V" | "A" | "W"): string {
  if (value === null) {
    return `--.-${unit}`;
  }
  return `${(value / 1000).toFixed(2)}${unit}`;
}

function PortStateSummary({
  powerEnabled,
  dataConnected,
  replugging,
}: {
  powerEnabled: boolean;
  dataConnected: boolean;
  replugging: boolean;
}) {
  const items = [
    {
      label: powerEnabled ? "Power on" : "Power off",
      active: powerEnabled,
    },
    {
      label: replugging
        ? "Replugging"
        : dataConnected
          ? "Data linked"
          : "Data off",
      active: dataConnected && !replugging,
    },
  ];

  return (
    <div className="grid h-7 grid-cols-2 gap-2">
      {items.map((item) => (
        <div
          className={[
            "flex min-w-0 items-center justify-center rounded-[8px] px-2 text-[11px] font-bold",
            item.active
              ? "bg-[var(--badge-success-bg)] text-[var(--badge-success-text)]"
              : "bg-[var(--btn-disabled-fill-soft)] text-[var(--muted)]",
          ].join(" ")}
          key={item.label}
        >
          <span className="truncate">{item.label}</span>
        </div>
      ))}
    </div>
  );
}

function ConfirmPopover({
  open,
  onClose,
  onConfirm,
}: {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    const onPointerDown = (e: PointerEvent) => {
      if (!ref.current) {
        return;
      }
      if (ref.current.contains(e.target as Node)) {
        return;
      }
      onClose();
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [onClose, open]);

  if (!open) {
    return null;
  }

  return (
    <div className="iso-popover absolute left-0 top-full z-50 mt-2" ref={ref}>
      <div className="relative">
        <div
          className="absolute left-[56px] top-[-6px] h-3 w-3 rotate-45 border border-[var(--border)] bg-[var(--panel)]"
          aria-hidden
        />
        <div className="flex h-[44px] w-[252px] items-center gap-2 rounded-[14px] border border-[var(--border)] bg-[var(--panel)] px-4">
          <div className="text-[12px] font-semibold text-[var(--muted)]">
            Power off?
          </div>
          <div className="flex-1" />
          <button
            className="flex h-6 w-11 items-center justify-center rounded-[8px] border border-[var(--border)] bg-transparent text-[12px] font-bold text-[var(--text)]"
            type="button"
            onClick={onClose}
          >
            No
          </button>
          <button
            className="flex h-6 w-11 items-center justify-center rounded-[8px] bg-[var(--primary)] text-[12px] font-extrabold text-[var(--primary-text)]"
            type="button"
            onClick={() => {
              onConfirm();
              onClose();
            }}
          >
            Yes
          </button>
        </div>
      </div>
    </div>
  );
}

export function PortCard({
  portId,
  label,
  telemetry,
  state,
  disabled,
  onTogglePower,
  onReplug,
}: PortCardProps) {
  const [confirmOffOpen, setConfirmOffOpen] = useState(false);
  const busy = state.busy;
  const actionDisabled = !!disabled || busy;
  const badge = statusBadgeStyles(telemetry.status);

  return (
    <div
      className="iso-card relative flex h-full min-h-[236px] flex-col rounded-[18px] bg-[var(--panel)] p-6 shadow-[inset_0_0_0_1px_var(--border)]"
      data-testid={`port-card-${portId}`}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0 text-[16px] font-bold">{label}</div>
        <div
          className={[
            "flex h-6 min-w-[60px] items-center justify-center rounded-full px-3",
            badge.bg,
            badge.text,
            "whitespace-nowrap text-[12px] font-semibold",
          ].join(" ")}
        >
          {telemetry.status === "not_inserted"
            ? "not inserted"
            : telemetry.status === "ok"
              ? "OK"
              : telemetry.status}
        </div>
      </div>

      <div className="mt-4 grid grid-cols-[76px_minmax(0,1fr)] items-center gap-3">
        <div className="text-[12px] font-semibold text-[var(--muted)]">
          State
        </div>
        <PortStateSummary
          powerEnabled={state.power_enabled}
          dataConnected={state.data_connected}
          replugging={state.replugging}
        />
      </div>

      <div className="mt-5 grid grid-cols-3 gap-6 sm:gap-10">
        <div>
          <div className="text-[12px] font-semibold text-[var(--muted)]">
            Voltage
          </div>
          <div className="mt-2 font-mono text-[24px] font-bold">
            {formatValue(telemetry.voltage_mv, "V")}
          </div>
        </div>
        <div>
          <div className="text-[12px] font-semibold text-[var(--muted)]">
            Current
          </div>
          <div className="mt-2 font-mono text-[24px] font-bold">
            {formatValue(telemetry.current_ma, "A")}
          </div>
        </div>
        <div>
          <div className="text-[12px] font-semibold text-[var(--muted)]">
            Power
          </div>
          <div className="mt-2 font-mono text-[24px] font-bold">
            {formatValue(telemetry.power_mw, "W")}
          </div>
        </div>
      </div>

      <div className="mt-6 flex flex-wrap items-center gap-3">
        <div className="relative w-full sm:w-auto">
          <button
            className={[
              "flex h-10 w-full items-center justify-center rounded-[10px] text-[12px] font-bold sm:w-[132px]",
              actionDisabled
                ? "bg-[var(--btn-disabled-fill)] text-[var(--btn-disabled-text)]"
                : "bg-[var(--primary)] text-[var(--primary-text)]",
            ].join(" ")}
            type="button"
            disabled={actionDisabled}
            onClick={() => {
              if (actionDisabled) {
                return;
              }
              if (state.power_enabled) {
                setConfirmOffOpen(true);
                return;
              }
              onTogglePower();
            }}
          >
            Power
          </button>
          <ConfirmPopover
            open={confirmOffOpen}
            onClose={() => setConfirmOffOpen(false)}
            onConfirm={onTogglePower}
          />
        </div>
        <button
          className={[
            "flex h-10 w-full items-center justify-center rounded-[10px] border border-[var(--border)] text-[12px] font-bold sm:w-[140px]",
            actionDisabled
              ? "bg-[var(--btn-disabled-fill-soft)] text-[var(--btn-disabled-text)]"
              : "bg-transparent text-[var(--text)]",
          ].join(" ")}
          type="button"
          disabled={actionDisabled}
          onClick={onReplug}
        >
          Replug
        </button>
      </div>
    </div>
  );
}
