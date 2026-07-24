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

function defaultDocsUrl(): string {
  const base = ((import.meta.env.BASE_URL as string | undefined) ?? "/").trim();
  const normalizedBase = base.endsWith("/") ? base : `${base}/`;
  return normalizedBase === "/" ? "/docs/" : `${normalizedBase}docs/`;
}

export function AboutPage() {
  const { sha, date, version } = buildInfo();
  const { agent, status } = useCompanionBridge();
  const { pushToast } = useToast();
  const [resetting, setResetting] = useState(false);

  const repoUrl = envLink("VITE_REPO_URL");
  const docsUrl = envLink("VITE_DOCS_URL") ?? defaultDocsUrl();
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
    <div className="flex flex-col gap-5" data-testid="about">
      <div className="iso-panel px-5 py-5 sm:px-6">
        <div className="text-[12px] font-semibold text-[var(--muted)]">
          Surface notes
        </div>
        <div className="mt-2 text-[30px] font-black leading-[0.94] tracking-[-0.03em]">
          About the control surface
        </div>
        <div className="mt-3 max-w-[72ch] text-[14px] font-medium leading-[1.6] text-[var(--muted)]">
          Build identity, repository routes, defaults, and local storage
          behavior for the operator shell.
        </div>
        <div className="mt-4 flex flex-wrap gap-x-4 gap-y-2 text-[12px] font-semibold text-[var(--muted)]">
          <span>Build identity</span>
          <span>Repo + docs links</span>
          <span>Local companion state</span>
        </div>
      </div>

      <div className="grid grid-cols-1 items-start gap-5 lg:grid-cols-2">
        <div className="iso-panel px-6 py-6">
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

        <div className="iso-panel px-6 py-6">
          <div className="text-[16px] font-bold leading-5">
            Links & defaults
          </div>

          <div className="mt-1 text-[12px] font-semibold leading-4 text-[var(--muted)]">
            Links
          </div>

          <div className="mt-1 flex flex-wrap items-center gap-2">
            <a
              className={[
                "iso-button w-[120px]",
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
                "iso-button w-[120px]",
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
                "iso-button w-[120px]",
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
          <div className="iso-panel px-6 py-6">
            <div className="text-[16px] font-bold leading-5">
              Local companion storage
            </div>
            <div className="mt-2 text-[12px] font-semibold text-[var(--muted)]">
              Devices + theme are stored in the local companion data directory.
            </div>
            <button
              className={[
                "iso-button mt-4",
                resetting || status !== "ready"
                  ? "[--iso-button-bg:var(--btn-disabled-fill-soft)] [--iso-button-text:var(--btn-disabled-text)]"
                  : "iso-button--ghost",
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

      <div className="iso-panel px-6 py-6">
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
