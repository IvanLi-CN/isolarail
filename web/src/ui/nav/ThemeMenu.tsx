import { useEffect, useRef, useState } from "react";

import type { ThemeId } from "../../app/theme";

const OPTIONS: Array<{ id: ThemeId; label: string }> = [
  { id: "isolarail", label: "isolarail" },
  { id: "isolarail-dark", label: "isolarail-dark" },
  { id: "system", label: "system" },
];

export function ThemeMenu({
  value,
  onChange,
}: {
  value: ThemeId;
  onChange: (next: ThemeId) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const prefersDark =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-color-scheme: dark)")?.matches === true;

  const buttonLabel =
    value === "system" ? (prefersDark ? "isolarail-dark" : "isolarail") : value;

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
      setOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  return (
    <div className="relative flex items-center gap-2" ref={ref}>
      <div className="iso-kicker text-[10px]">Theme</div>
      <button
        className="iso-button w-[172px] justify-between px-4"
        type="button"
        onClick={() => setOpen((v) => !v)}
      >
        <span>{buttonLabel}</span>
        <span aria-hidden="true">▾</span>
      </button>
      {open ? (
        <div
          className={[
            "iso-popover absolute right-0 top-full z-50 mt-2",
            "iso-panel w-[220px] p-2",
          ].join(" ")}
          role="menu"
        >
          {OPTIONS.map((opt) => (
            <button
              key={opt.id}
              className={[
                "flex w-full items-center justify-between rounded-[12px] border px-3 py-2 text-left text-[12px] font-semibold",
                opt.id === value
                  ? "border-[color-mix(in_srgb,var(--primary)_30%,var(--border))] bg-[var(--badge-signal-bg)] text-[var(--badge-signal-text)]"
                  : "border-transparent bg-transparent text-[var(--text)]",
              ].join(" ")}
              type="button"
              onClick={() => {
                onChange(opt.id);
                setOpen(false);
              }}
            >
              <span>{opt.label}</span>
              {opt.id === value ? (
                <span className="text-[var(--muted)]">✓</span>
              ) : null}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
