import { NavLink } from "react-router";

export function DevicePageTabs({ deviceId }: { deviceId: string }) {
  return (
    <div
      className="flex flex-wrap items-center gap-2"
      role="tablist"
      data-testid="device-tabs"
    >
      <NavLink
        className={({ isActive }) =>
          [
            "iso-button w-[132px] text-[12px]",
            isActive
              ? "iso-button--signal-soft"
              : "[--iso-button-bg:var(--tab-inactive-bg)] [--iso-button-text:var(--muted)]",
          ].join(" ")
        }
        to={`/devices/${deviceId}`}
        role="tab"
        end
      >
        Dashboard
      </NavLink>
      <NavLink
        className={({ isActive }) =>
          [
            "iso-button w-[132px] text-[12px]",
            isActive
              ? "iso-button--signal-soft"
              : "[--iso-button-bg:var(--tab-inactive-bg)] [--iso-button-text:var(--muted)]",
          ].join(" ")
        }
        to={`/devices/${deviceId}/settings`}
        role="tab"
      >
        Settings
      </NavLink>
      <NavLink
        className={({ isActive }) =>
          [
            "iso-button w-[132px] text-[12px]",
            isActive
              ? "iso-button--signal-soft"
              : "[--iso-button-bg:var(--tab-inactive-bg)] [--iso-button-text:var(--muted)]",
          ].join(" ")
        }
        to={`/devices/${deviceId}/debug/hardware`}
        role="tab"
      >
        Debug
      </NavLink>
      <NavLink
        className={({ isActive }) =>
          [
            "iso-button w-[132px] text-[12px]",
            isActive
              ? "iso-button--signal-soft"
              : "[--iso-button-bg:var(--tab-inactive-bg)] [--iso-button-text:var(--muted)]",
          ].join(" ")
        }
        to={`/devices/${deviceId}/info`}
        role="tab"
      >
        Info
      </NavLink>
    </div>
  );
}
