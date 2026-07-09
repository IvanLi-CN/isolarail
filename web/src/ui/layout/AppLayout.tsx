import type { ReactNode } from "react";
import { Link, useLocation } from "react-router";
import { useTheme } from "../../app/theme-ui";
import { ThemeMenu } from "../nav/ThemeMenu";

export function AppLayout({
  sidebar,
  children,
}: {
  sidebar: ReactNode;
  children: ReactNode;
}) {
  const { theme, setTheme } = useTheme();
  const location = useLocation();

  const showTheme = location.pathname === "/" || location.pathname === "/about";

  return (
    <div className="min-h-screen">
      <header className="border-b border-[var(--border)] px-4 pb-4 pt-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-[1680px]">
          <div className="iso-panel-subtle flex flex-col gap-4 px-4 py-4 lg:flex-row lg:items-center lg:justify-between lg:px-5">
            <Link className="min-w-0 no-underline" to="/">
              <div className="flex items-start gap-3">
                <span
                  aria-hidden="true"
                  className="iso-brand-mark mt-[6px] shrink-0"
                />
                <div className="min-w-0">
                  <div className="iso-kicker">IsolaRail Control</div>
                  <div className="mt-2 text-[28px] font-black leading-[0.96] tracking-[-0.05em] text-[var(--text)]">
                    Relay Console
                  </div>
                  <div className="mt-2 max-w-[52ch] text-[13px] font-medium leading-[1.55] text-[var(--muted)]">
                    Device discovery, route control, and measured power
                    telemetry in one operator surface.
                  </div>
                </div>
              </div>
            </Link>
            <div className="flex flex-wrap items-center gap-2">
              {showTheme ? (
                <div className="w-full sm:w-auto">
                  <ThemeMenu value={theme} onChange={setTheme} />
                </div>
              ) : null}
              <Link className="iso-button iso-button--ghost" to="/about">
                About Surface
              </Link>
            </div>
          </div>
        </div>
      </header>
      <div className="mx-auto flex w-full max-w-[1680px] flex-col gap-5 px-4 pb-5 pt-5 sm:px-6 lg:px-8 xl:min-h-[calc(100vh-148px)] xl:flex-row">
        <aside className="w-full shrink-0 xl:w-[372px]">
          <div className="iso-panel-subtle h-full overflow-hidden bg-[var(--sidebar-bg)]">
            {sidebar}
          </div>
        </aside>
        <main className="min-w-0 flex-1">
          <div className="flex h-full min-h-[560px] flex-col gap-5">
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
