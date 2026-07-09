import { useEffect, useRef, useState } from "react";

import type { PortId, PortState, PortTelemetry } from "../../domain/ports";

function formatValue(value: number | null, unit: "V" | "A" | "W"): string {
  if (value === null) {
    return `--.-${unit}`;
  }
  return `${(value / 1000).toFixed(2)}${unit}`;
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
    <div className="iso-popover absolute left-0 top-full z-50 mt-2">
      <div className="relative">
        <div
          className="absolute left-[40px] top-[-6px] h-3 w-3 rotate-45 border border-[var(--border)] bg-[var(--panel)]"
          aria-hidden
        />
        <div className="iso-panel flex h-[52px] w-[272px] items-center gap-2 px-4">
          <div className="text-[12px] font-semibold text-[var(--muted)]">
            Power off?
          </div>
          <div className="flex-1" />
          <button
            className="iso-button h-7 w-12 px-0 text-[11px]"
            type="button"
            onClick={onClose}
          >
            No
          </button>
          <button
            className="iso-button iso-button--primary h-7 w-12 px-0 text-[11px]"
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

export type PortMiniCardProps = {
  portId: PortId;
  label: string;
  telemetry: PortTelemetry;
  state: PortState;
  disabled: boolean;
  compact?: boolean;
  className?: string;
  onSetPower: (enabled: boolean) => void;
  onReplug: () => void;
};

export function PortMiniCard({
  portId,
  label,
  telemetry,
  state,
  disabled,
  compact = false,
  className,
  onSetPower,
  onReplug,
}: PortMiniCardProps) {
  const [confirmOffOpen, setConfirmOffOpen] = useState(false);

  const busy = state.busy;
  const powerEnabled = state.power_enabled;
  const actionDisabled = disabled || busy;

  const powerWidth = compact ? "w-[84px]" : "w-[100px]";
  const replugWidth = compact
    ? "w-[84px]"
    : portId === "port4"
      ? "w-[104px]"
      : "w-[112px]";

  const valueClass = [
    compact ? "text-[14px]" : "text-[16px]",
    "font-bold",
    "font-mono",
    actionDisabled ? "text-[var(--muted)]" : "text-[var(--text)]",
  ].join(" ");

  return (
    <div
      className={[
        compact
          ? "relative rounded-[14px] border border-[var(--border)] bg-[var(--panel)] px-4 py-3"
          : "relative h-[132px] rounded-[16px] border border-[var(--border)] bg-[var(--panel)] px-5 py-4",
        className ?? "",
      ].join(" ")}
    >
      <div className="flex items-center gap-2">
        <div
          className={
            compact
              ? "text-[11px] font-semibold text-[var(--muted)]"
              : "text-[12px] font-semibold text-[var(--muted)]"
          }
        >
          {label}
        </div>
      </div>
      <div
        className={
          compact
            ? "mt-3 flex items-center justify-between gap-3"
            : "mt-4 flex items-center justify-between gap-4"
        }
      >
        <div className={valueClass}>
          {formatValue(telemetry.voltage_mv, "V")}
        </div>
        <div className={valueClass}>
          {formatValue(telemetry.current_ma, "A")}
        </div>
        <div className={valueClass}>{formatValue(telemetry.power_mw, "W")}</div>
      </div>
      <div
        className={
          compact
            ? "mt-3 flex items-center gap-2"
            : "mt-[18px] flex items-center gap-2"
        }
      >
        <div className="relative">
          <button
            className={[
              compact
                ? "iso-button h-[32px] text-[11px]"
                : "iso-button h-[34px] text-[12px]",
              powerWidth,
              actionDisabled
                ? "[--iso-button-bg:var(--btn-disabled-fill)] [--iso-button-border:var(--border)] [--iso-button-text:var(--btn-disabled-text)]"
                : "iso-button--primary",
            ].join(" ")}
            type="button"
            disabled={actionDisabled}
            onClick={() => {
              if (actionDisabled) {
                return;
              }
              if (powerEnabled) {
                setConfirmOffOpen(true);
                return;
              }
              onSetPower(true);
            }}
          >
            Power
          </button>
          <ConfirmPopover
            open={confirmOffOpen}
            onClose={() => setConfirmOffOpen(false)}
            onConfirm={() => onSetPower(false)}
          />
        </div>
        <button
          className={[
            compact
              ? "iso-button h-[32px] text-[11px]"
              : "iso-button h-[34px] text-[12px]",
            replugWidth,
            actionDisabled
              ? "[--iso-button-bg:var(--btn-disabled-fill-soft)] [--iso-button-border:var(--border)] [--iso-button-text:var(--btn-disabled-text)]"
              : "iso-button--ghost",
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
