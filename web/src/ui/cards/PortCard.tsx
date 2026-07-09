import { Circle, LoaderCircle, Power, RotateCw, Zap } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { PortCardProps } from "./types";

function statusBadgeStyles(status: string): { tone: string } {
  if (status === "ok") {
    return {
      tone: "iso-chip--success",
    };
  }
  if (status === "error") {
    return {
      tone: "iso-chip--error",
    };
  }
  return {
    tone: "iso-chip--warning",
  };
}

function statusLabel(status: string): string {
  if (status === "ok") {
    return "OK";
  }
  if (status === "off") {
    return "power off";
  }
  if (status === "not_inserted") {
    return "not inserted";
  }
  return status;
}

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
    <div className="iso-popover absolute left-0 top-full z-50 mt-2" ref={ref}>
      <div className="relative">
        <div
          className="absolute left-[56px] top-[-6px] h-3 w-3 rotate-45 border border-[var(--border)] bg-[var(--panel)]"
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

export function PortCard({
  portId,
  label,
  telemetry,
  state,
  disabled,
  powerPending = false,
  onTogglePower,
  onReplug,
}: PortCardProps) {
  const [confirmOffOpen, setConfirmOffOpen] = useState(false);
  const [powerPulse, setPowerPulse] = useState(false);
  const [replugPulse, setReplugPulse] = useState(false);
  const busy = state.busy;
  const actionDisabled = !!disabled || busy;
  const badge = statusBadgeStyles(telemetry.status);
  const powerEnabled = state.power_enabled;
  const powerAnimating = powerPulse || powerPending;

  const triggerPowerToggle = () => {
    setPowerPulse(false);
    window.requestAnimationFrame(() => setPowerPulse(true));
    onTogglePower();
  };

  useEffect(() => {
    if (!replugPulse) {
      return;
    }
    const id = window.setTimeout(() => setReplugPulse(false), 280);
    return () => window.clearTimeout(id);
  }, [replugPulse]);

  useEffect(() => {
    if (!powerPulse) {
      return;
    }
    const id = window.setTimeout(() => setPowerPulse(false), 280);
    return () => window.clearTimeout(id);
  }, [powerPulse]);

  return (
    <div
      className="iso-panel relative flex h-full min-h-[248px] flex-col p-6"
      data-testid={`port-card-${portId}`}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0 text-[18px] font-black tracking-[-0.03em]">
          {label}
        </div>
        <div
          className={[
            "iso-chip h-7 min-w-[72px] items-center justify-center px-3",
            badge.tone,
            "whitespace-nowrap text-[11px]",
          ].join(" ")}
        >
          {statusLabel(telemetry.status)}
        </div>
      </div>

      <div className="mt-6 grid grid-cols-3 gap-3">
        <div className="rounded-[14px] border border-[var(--border)] bg-[var(--panel-2)] px-3 py-3">
          <div className="text-[12px] font-semibold text-[var(--muted)]">
            Voltage
          </div>
          <div className="mt-2 font-mono text-[24px] font-bold">
            {formatValue(telemetry.voltage_mv, "V")}
          </div>
        </div>
        <div className="rounded-[14px] border border-[var(--border)] bg-[var(--panel-2)] px-3 py-3">
          <div className="text-[12px] font-semibold text-[var(--muted)]">
            Current
          </div>
          <div className="mt-2 font-mono text-[24px] font-bold">
            {formatValue(telemetry.current_ma, "A")}
          </div>
        </div>
        <div className="rounded-[14px] border border-[var(--border)] bg-[var(--panel-2)] px-3 py-3">
          <div className="text-[12px] font-semibold text-[var(--muted)]">
            Power
          </div>
          <div className="mt-2 font-mono text-[24px] font-bold">
            {formatValue(telemetry.power_mw, "W")}
          </div>
        </div>
      </div>

      <div className="mt-7 flex flex-wrap items-center gap-3">
        <div className="relative min-w-[220px] flex-1 sm:max-w-[252px]">
          <button
            className={[
              "group flex h-12 w-full items-center gap-3 rounded-[12px] border px-3 text-left transition-colors duration-150",
              actionDisabled
                ? "border-[var(--border)] bg-[var(--btn-disabled-fill)] text-[var(--btn-disabled-text)]"
                : powerEnabled
                  ? "border-[var(--badge-success-bg)] bg-[var(--badge-success-bg)] text-[var(--badge-success-text)]"
                  : "border-[var(--border)] bg-[var(--btn-disabled-fill-soft)] text-[var(--muted)]",
              powerPulse ? "iso-control-pulse" : "",
            ].join(" ")}
            type="button"
            disabled={actionDisabled}
            aria-label={
              powerEnabled ? "Power on, turn off" : "Power off, turn on"
            }
            title={powerEnabled ? "Turn power off" : "Turn power on"}
            onClick={() => {
              if (actionDisabled) {
                return;
              }
              if (state.power_enabled) {
                setConfirmOffOpen(true);
                return;
              }
              triggerPowerToggle();
            }}
          >
            <span
              className={[
                "flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] border",
                powerEnabled
                  ? "border-[var(--badge-success-text)] bg-[var(--badge-success-text)] text-[var(--panel)]"
                  : "border-[var(--power-track-off)] bg-[var(--btn-disabled-fill-soft)] text-[var(--muted)]",
              ].join(" ")}
              aria-hidden
            >
              {powerAnimating ? (
                <LoaderCircle
                  className="iso-control-spin"
                  size={15}
                  strokeWidth={2.4}
                />
              ) : (
                <Power size={15} strokeWidth={2.4} />
              )}
            </span>
            <span className="min-w-0 flex-1">
              <span className="flex items-center gap-2">
                {powerEnabled ? (
                  <Zap size={13} strokeWidth={2.2} aria-hidden />
                ) : (
                  <Circle size={12} strokeWidth={2.2} aria-hidden />
                )}
                <span className="text-[11px] font-extrabold tracking-[0.08em]">
                  {powerEnabled ? "ON" : "OFF"}
                </span>
              </span>
              <span
                className={[
                  "mt-[2px] block h-1 w-full max-w-[92px] rounded-full",
                  powerEnabled
                    ? "bg-[var(--badge-success-text)]"
                    : "bg-[var(--power-track-off)]",
                ].join(" ")}
                aria-hidden
              />
            </span>
            <span
              className={[
                "iso-chip h-7 min-w-[54px] items-center justify-center px-2 text-[10px]",
                powerEnabled ? "iso-chip--success" : "iso-chip--neutral",
                actionDisabled
                  ? "[--iso-chip-bg:var(--btn-disabled-fill-soft)] [--iso-chip-border:var(--border)] [--iso-chip-text:var(--btn-disabled-text)]"
                  : "",
              ].join(" ")}
            >
              {powerEnabled ? "Cut" : "Restore"}
            </span>
          </button>
          <ConfirmPopover
            open={confirmOffOpen}
            onClose={() => setConfirmOffOpen(false)}
            onConfirm={triggerPowerToggle}
          />
        </div>
        <button
          className={[
            "iso-button h-12 w-full gap-2 text-[12px] transition-colors duration-150 sm:w-[112px]",
            actionDisabled
              ? "[--iso-button-bg:var(--btn-disabled-fill-soft)] [--iso-button-border:var(--border)] [--iso-button-text:var(--btn-disabled-text)]"
              : state.replugging
                ? "[--iso-button-bg:var(--btn-disabled-fill-soft)] [--iso-button-border:var(--primary)] [--iso-button-text:var(--primary)]"
                : "iso-button--ghost",
            replugPulse ? "iso-control-pulse" : "",
          ].join(" ")}
          type="button"
          disabled={actionDisabled}
          onClick={() => {
            if (actionDisabled) {
              return;
            }
            setReplugPulse(false);
            window.requestAnimationFrame(() => setReplugPulse(true));
            onReplug();
          }}
          title="Replug USB data path"
        >
          <RotateCw
            className={
              state.replugging || replugPulse ? "iso-control-spin" : undefined
            }
            size={14}
            strokeWidth={2.2}
            aria-hidden
          />
          Replug
        </button>
        {state.replugging ? (
          <div className="iso-chip h-8 items-center gap-2 px-3 text-[11px] [--iso-chip-bg:var(--btn-disabled-fill-soft)] [--iso-chip-border:var(--border)] [--iso-chip-text:var(--muted)]">
            <RotateCw
              className="iso-control-spin"
              size={12}
              strokeWidth={2.2}
              aria-hidden
            />
            Replugging
          </div>
        ) : null}
      </div>
    </div>
  );
}
