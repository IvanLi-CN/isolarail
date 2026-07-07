import { useState } from "react";
import { useCompanionBridge } from "../app/companion-bridge-ui";
import { resetStorage } from "../domain/companionStorage";
import { useToast } from "../ui/toast/ToastProvider";

function buildInfo(): { sha: string; date: string; version: string } {
  const rawSha =
    (import.meta.env.VITE_BUILD_SHA as string | undefined) ?? "dev";
  const sha = rawSha === "dev" ? rawSha : rawSha.slice(0, 7);
  const date = (import.meta.env.VITE_BUILD_DATE as string | undefined) ?? "";
  const version =
    (import.meta.env.VITE_RELEASE_VERSION as string | undefined) ?? "dev";
  return { sha, date, version };
}

function envLink(key: string): string | null {
  const value = (import.meta.env[key] as string | undefined) ?? "";
  return value.trim() ? value.trim() : null;
}

export function AboutPage() {
  const { sha, date, version } = buildInfo();
  const { agent, status } = useCompanionBridge();
  const { pushToast } = useToast();
  const [resetting, setResetting] = useState(false);

  const repoUrl = envLink("VITE_REPO_URL");
  const docsUrl = envLink("VITE_DOCS_URL");
  const issuesUrl = envLink("VITE_ISSUES_URL");

  const onResetStorage = async () => {
    if (!agent || status !== "ready" || resetting) {
      return;
    }
    const confirmed = window.confirm(
      "Reset local data? This clears saved devices and theme in the local companion storage.",
    );
    if (!confirmed) {
      return;
    }
    setResetting(true);
    const res = await resetStorage(agent);
    setResetting(false);
    if (res.ok) {
      pushToast({ variant: "success", message: "Local data reset." });
      return;
    }
    pushToast({
      variant: "error",
      message: `Reset failed: ${res.error.message}`,
    });
  };

  return (
    <div className="flex flex-col gap-6" data-testid="about">
      <div>
        <div className="text-[24px] font-bold">About</div>
        <div className="mt-2 text-[14px] font-medium text-[var(--muted)]">
          Build info, links, and quick usage
        </div>
      </div>

      <div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-2">
        <div className="iso-card rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
          <div className="text-[16px] font-bold leading-5">Build</div>

          <div className="mt-3 flex flex-col gap-[10px] leading-4">
            <div className="flex items-center">
              <div className="w-[54px] text-[12px] font-semibold text-[var(--muted)]">
                version
              </div>
              <div className="font-mono text-[12px] font-semibold">
                {version}
              </div>
            </div>
            <div className="flex items-center">
              <div className="w-[54px] text-[12px] font-semibold text-[var(--muted)]">
                build
              </div>
              <div className="font-mono text-[12px] font-semibold">{sha}</div>
            </div>
            <div className="flex items-center">
              <div className="w-[54px] text-[12px] font-semibold text-[var(--muted)]">
                date
              </div>
              <div className="font-mono text-[12px] font-semibold">
                {date || "unknown"}
              </div>
            </div>
            <div className="flex items-center">
              <div className="w-[54px] text-[12px] font-semibold text-[var(--muted)]">
                theme
              </div>
              <div className="text-[12px] font-semibold">
                isolarail / isolarail-dark / system
              </div>
            </div>
          </div>
        </div>

        <div className="iso-card rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
          <div className="text-[16px] font-bold leading-5">
            Links & defaults
          </div>

          <div className="mt-1 text-[12px] font-semibold leading-4 text-[var(--muted)]">
            Links
          </div>

          <div className="mt-1 flex flex-wrap items-center gap-2">
            <a
              className={[
                "flex h-9 w-[120px] items-center justify-center rounded-[10px] border border-[var(--border)] bg-transparent text-[12px] font-bold text-[var(--text)]",
                repoUrl ? "" : "pointer-events-none opacity-40",
              ].join(" ")}
              href={repoUrl ?? undefined}
              target="_blank"
              rel="noreferrer"
            >
              Repo
            </a>
            <a
              className={[
                "flex h-9 w-[120px] items-center justify-center rounded-[10px] border border-[var(--border)] bg-transparent text-[12px] font-bold text-[var(--text)]",
                docsUrl ? "" : "pointer-events-none opacity-40",
              ].join(" ")}
              href={docsUrl ?? undefined}
              target="_blank"
              rel="noreferrer"
            >
              Docs
            </a>
            <a
              className={[
                "flex h-9 w-[120px] items-center justify-center rounded-[10px] border border-[var(--border)] bg-transparent text-[12px] font-bold text-[var(--text)]",
                issuesUrl ? "" : "pointer-events-none opacity-40",
              ].join(" ")}
              href={issuesUrl ?? undefined}
              target="_blank"
              rel="noreferrer"
            >
              Issues
            </a>
          </div>

          <div className="mt-0 text-[12px] font-semibold leading-4 text-[var(--muted)]">
            Defaults
          </div>
          <div className="grid grid-cols-1 gap-x-6 gap-y-[6px] leading-4 sm:grid-cols-2">
            <div className="text-[12px] font-semibold">Units: V / A / W</div>
            <div className="text-[12px] font-semibold">Power off: confirm</div>
            <div className="text-[12px] font-semibold">
              Report: 1s • Offline: 10s
            </div>
            <div className="text-[12px] font-semibold">Replug: one-shot</div>
          </div>
        </div>

        {agent ? (
          <div className="iso-card rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
            <div className="text-[16px] font-bold leading-5">
              Local companion storage
            </div>
            <div className="mt-2 text-[12px] font-semibold text-[var(--muted)]">
              Devices + theme are stored in the local companion data directory.
            </div>
            <button
              className={[
                "mt-4 h-9 rounded-[10px] border border-[var(--border)] px-4 text-[12px] font-bold",
                "hover:bg-[var(--panel-2)]",
              ].join(" ")}
              type="button"
              disabled={resetting || status !== "ready"}
              onClick={onResetStorage}
            >
              {resetting ? "Resetting..." : "Reset local data"}
            </button>
          </div>
        ) : null}
      </div>

      <div className="iso-card rounded-[18px] bg-[var(--panel)] px-6 py-6 shadow-[inset_0_0_0_1px_var(--border)]">
        <div className="text-[16px] font-bold">Quick usage</div>

        <div className="mt-4 text-[14px] font-medium">
          1) Add a device: baseUrl examples
        </div>
        <div className="mt-[10px] space-y-[6px] font-mono text-[12px] font-semibold text-[var(--muted)] leading-4">
          <div>http://&lt;hostname&gt;.local</div>
          <div>http://192.168.1.42</div>
        </div>

        <div className="mt-6 text-[14px] font-medium">
          2) Dashboard shows V/A/W and actions
        </div>
        <div className="mt-4 text-[14px] font-medium">
          3) Power off requires a popover confirmation
        </div>

        <div className="mt-8 flex items-center leading-4">
          <div className="w-[54px] text-[12px] font-semibold text-[var(--muted)]">
            Language
          </div>
          <div className="text-[12px] font-semibold">
            Default English; i18n later
          </div>
        </div>
      </div>
    </div>
  );
}
