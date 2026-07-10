import { useEffect, useRef, useState } from "react";

import type { PortId, PortState, PortTelemetry } from "../../domain/ports";

function formatValue(value: number | null, unit: "V" | "A" | "W"): string {
  if (value === null) {
    return `--.-${unit}`;
  }
  return `${(value / 1000).toFixed(2)}${unit}`;
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
  const powerButtonRef = useRef<HTMLButtonElement>(null);
  const confirmCancelRef = useRef<HTMLButtonElement>(null);

  const busy = state.busy;
  const powerEnabled = state.power_enabled;
  const actionDisabled = disabled || busy;

  const closeConfirm = () => {
    setConfirmOffOpen(false);
    window.requestAnimationFrame(() => powerButtonRef.current?.focus());
  };

  useEffect(() => {
    if (!confirmOffOpen) {
      return;
    }
    confirmCancelRef.current?.focus();
  }, [confirmOffOpen]);

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
            ref={powerButtonRef}
            className={[
              compact
                ? "iso-button h-11 text-[11px]"
                : "iso-button h-11 text-[12px]",
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
        </div>
        <button
          className={[
            compact
              ? "iso-button h-11 text-[11px]"
              : "iso-button h-11 text-[12px]",
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
      {confirmOffOpen ? (
        <fieldset
          aria-describedby={`${portId}-mini-power-confirm-description`}
          className="mt-3 rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-3 py-3"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              closeConfirm();
            }
          }}
        >
          <legend
            className="text-[11px] font-extrabold uppercase tracking-[0.08em] text-[var(--text)]"
            id={`${portId}-mini-power-confirm-title`}
          >
            Cut power to {label}?
          </legend>
          <div
            className="mt-1 text-[12px] font-semibold leading-[1.55] text-[var(--muted)]"
            id={`${portId}-mini-power-confirm-description`}
          >
            The rail stays off until you restore it.
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            <button
              ref={confirmCancelRef}
              className="iso-button iso-button--ghost min-h-[44px]"
              type="button"
              onClick={closeConfirm}
            >
              Cancel
            </button>
            <button
              className="iso-button iso-button--primary min-h-[44px]"
              type="button"
              onClick={() => {
                onSetPower(false);
                closeConfirm();
              }}
            >
              Cut power
            </button>
          </div>
        </fieldset>
      ) : null}
    </div>
  );
}
