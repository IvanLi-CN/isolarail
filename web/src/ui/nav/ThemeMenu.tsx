import type { ThemeId } from "../../app/theme";

const OPTIONS: Array<{ id: ThemeId; label: string }> = [
  { id: "isolarail", label: "Light" },
  { id: "isolarail-dark", label: "Dark" },
  { id: "system", label: "System" },
];

export function ThemeMenu({
  value,
  onChange,
}: {
  value: ThemeId;
  onChange: (next: ThemeId) => void;
}) {
  const selectId = "isolarail-theme-select";

  return (
    <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
      <label
        className="text-[12px] font-semibold text-[var(--muted)]"
        htmlFor={selectId}
      >
        Theme
      </label>
      <div className="relative min-w-[172px]">
        <select
          className="iso-select pr-10 text-[13px] font-semibold"
          id={selectId}
          value={value}
          onChange={(event) => onChange(event.target.value as ThemeId)}
        >
          {OPTIONS.map((opt) => (
            <option key={opt.id} value={opt.id}>
              {opt.label}
            </option>
          ))}
        </select>
        <span
          aria-hidden="true"
          className="pointer-events-none absolute inset-y-0 right-4 flex items-center text-[var(--muted)]"
        >
          ▾
        </span>
      </div>
    </div>
  );
}
