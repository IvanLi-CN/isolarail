import { useEffect, useRef, useState } from "react";

import type { ThemeId } from "../../app/theme";

const OPTIONS: Array<{ id: ThemeId; label: string }> = [
  { id: "isohub", label: "isohub" },
  { id: "isohub-dark", label: "isohub-dark" },
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
    value === "system" ? (prefersDark ? "isohub-dark" : "isohub") : value;

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
      <div className="text-[12px] font-semibold text-[var(--muted)]">Theme</div>
      <button
        className="flex h-9 w-[150px] items-center rounded-[10px] border border-[var(--border)] bg-transparent px-5 text-[12px] font-bold text-[var(--text)]"
        type="button"
        onClick={() => setOpen((v) => !v)}
      >
        {buttonLabel} ▾
      </button>
      {open ? (
        <div
          className={[
            "iso-popover absolute right-0 top-full z-50 mt-2",
            "w-[200px] rounded-[14px] border border-[var(--border)] bg-[var(--panel)] p-2",
          ].join(" ")}
          role="menu"
        >
          {OPTIONS.map((opt) => (
            <button
              key={opt.id}
              className={[
                "flex w-full items-center justify-between rounded-[10px] px-3 py-2 text-left text-[12px] font-semibold",
                opt.id === value ? "bg-[var(--panel-2)]" : "bg-transparent",
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
