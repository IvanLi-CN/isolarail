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
    <div className="flex min-h-screen flex-col">
      <header className="h-16 border-b border-[var(--border)] bg-[var(--panel-2)]">
        <div className="mx-auto flex h-full max-w-[1600px] items-center justify-between gap-3 px-4 sm:px-6 lg:px-8">
          <Link className="min-w-0 truncate text-[16px] font-bold" to="/">
            IsoHub Control
          </Link>
          <div className="flex items-center gap-3">
            {showTheme ? (
              <div className="hidden sm:block">
                <ThemeMenu value={theme} onChange={setTheme} />
              </div>
            ) : null}
            <Link
              className="flex h-9 shrink-0 items-center justify-center rounded-[10px] border border-[var(--border)] bg-transparent px-3 text-[12px] font-bold text-[var(--text)] sm:px-4"
              to="/about"
            >
              About
            </Link>
          </div>
        </div>
      </header>
      <div className="flex min-h-0 flex-1 overflow-x-hidden">
        <div className="mx-auto flex w-full min-h-0 max-w-[1600px] flex-col xl:flex-row xl:overflow-hidden">
          <aside className="w-full min-h-0 shrink-0 border-b border-[var(--border)] bg-[var(--sidebar-bg)] xl:w-[360px] xl:overflow-y-auto xl:border-b-0 xl:border-r">
            {sidebar}
          </aside>
          <main className="min-h-0 min-w-0 flex-1 px-4 py-6 sm:px-6 lg:px-8 xl:overflow-y-auto">
            {children}
          </main>
        </div>
      </div>
    </div>
  );
}
