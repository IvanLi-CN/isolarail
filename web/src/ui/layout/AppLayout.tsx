import type { ReactNode } from "react";
import { Link } from "react-router";
import { useTheme } from "../../app/theme-ui";
import { ThemeMenu } from "../nav/ThemeMenu";

export function AppLayout({
  sidebar,
  children,
}: {
  sidebar?: ReactNode;
  children: ReactNode;
}) {
  const { theme, setTheme } = useTheme();
  const showSidebar = Boolean(sidebar);

  return (
    <div className="min-h-screen">
      <header className="border-b border-[var(--border)] px-4 py-3 sm:px-6 lg:px-8">
        <div className="mx-auto flex max-w-[1680px] flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <Link className="min-w-0 no-underline" to="/">
            <div className="flex items-center gap-3">
              <span aria-hidden="true" className="iso-brand-mark shrink-0" />
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                  <div className="text-[18px] font-black tracking-[-0.03em] text-[var(--text)]">
                    IsolaRail
                  </div>
                  <div className="text-[12px] font-semibold text-[var(--muted)]">
                    Relay Console
                  </div>
                </div>
                <div className="mt-1 hidden max-w-[56ch] text-[12px] font-medium leading-[1.5] text-[var(--muted)] sm:block">
                  Device discovery, route control, and measured power telemetry
                  in one operator surface.
                </div>
              </div>
            </div>
          </Link>
          <div className="flex flex-wrap items-center gap-2 sm:gap-3">
            <div className="w-full sm:w-auto">
              <ThemeMenu value={theme} onChange={setTheme} />
            </div>
            <Link className="iso-button iso-button--ghost" to="/about">
              About Surface
            </Link>
          </div>
        </div>
      </header>
      <div
        className={[
          "mx-auto flex w-full max-w-[1680px] flex-col gap-5 px-4 pb-5 pt-5 sm:px-6 lg:px-8",
          showSidebar ? "xl:min-h-[calc(100vh-148px)] xl:flex-row" : "",
        ].join(" ")}
      >
        {showSidebar ? (
          <aside className="w-full shrink-0 xl:w-[372px]">
            <div className="iso-panel-subtle h-full overflow-hidden bg-[var(--sidebar-bg)]">
              {sidebar}
            </div>
          </aside>
        ) : null}
        <main className="min-w-0 flex-1">
          <div className="flex h-full min-h-[560px] flex-col gap-5">
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
