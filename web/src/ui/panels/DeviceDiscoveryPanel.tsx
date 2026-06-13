import { useEffect, useMemo, useState } from "react";

import type {
  DiscoveredDevice,
  DiscoverySnapshot,
} from "../../domain/discovery";
import {
  isDiscoveredDeviceAdded,
  validateCidrInput,
} from "../../domain/discovery";

function Chevron({ direction }: { direction: "up" | "down" }) {
  const d = direction === "down" ? "M6 9l6 6 6-6" : "M6 15l6-6 6 6";
  return (
    <svg
      aria-hidden="true"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      className="opacity-70"
    >
      <path d={d} fill="none" stroke="currentColor" strokeWidth="2" />
    </svg>
  );
}

function unknown(value: string | undefined | null): string {
  if (!value || value.trim().length === 0) {
    return "unknown";
  }
  return value;
}

export type DeviceDiscoveryPanelProps = {
  snapshot: DiscoverySnapshot;
  existingDeviceIds: string[];
  existingDeviceBaseUrls: string[];
  onRefresh: () => void;
  onToggleIpScan: (expanded: boolean) => void;
  onStartScan: (cidr: string) => void;
  onCancelScan: () => void;
  onSelect: (device: DiscoveredDevice) => void;
};

export function DeviceDiscoveryPanel({
  snapshot,
  existingDeviceIds,
  existingDeviceBaseUrls,
  onRefresh,
  onToggleIpScan,
  onStartScan,
  onCancelScan,
  onSelect,
}: DeviceDiscoveryPanelProps) {
  const [filter, setFilter] = useState("");
  const [cidr, setCidr] = useState("");
  const [cidrTouched, setCidrTouched] = useState(false);
  const [cidrError, setCidrError] = useState<string | null>(null);

  useEffect(() => {
    if (snapshot.status !== "scanning") {
      setCidrError(null);
    }
  }, [snapshot.status]);

  useEffect(() => {
    if (cidrTouched || cidr.trim().length > 0) {
      return;
    }
    const next = snapshot.ipScan?.defaultCidr;
    if (next && next.trim().length > 0) {
      setCidr(next);
    }
  }, [cidr, cidrTouched, snapshot.ipScan?.defaultCidr]);

  const filteredDevices = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) {
      return snapshot.devices;
    }
    return snapshot.devices.filter((d) => {
      const fields = [d.hostname, d.fqdn, d.device_id, d.ipv4, d.baseUrl]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return fields.includes(q);
    });
  }, [filter, snapshot.devices]);

  const ipScanExpanded = snapshot.ipScan?.expanded ?? false;
  const ipScanCandidates = snapshot.ipScan?.candidates;
  const ipScanCandidatesList = ipScanCandidates ?? [];
  const scanning = snapshot.status === "scanning" && snapshot.mode === "scan";
  const emptyLabel =
    snapshot.status === "scanning"
      ? "Scanning…"
      : snapshot.status === "ready"
        ? "No devices found."
        : "No devices yet.";

  const startScan = () => {
    const res = validateCidrInput(cidr);
    if (!res.ok) {
      setCidrError(res.error);
      return;
    }
    setCidrError(null);
    onStartScan(res.cidr);
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="text-[16px] font-bold">Auto discovery</div>
          <div className="mt-2 text-[12px] font-semibold text-[var(--muted)]">
            Find hubs on your local network; pick one to add it.
          </div>
        </div>
        <button
          className="btn btn-sm"
          type="button"
          onClick={onRefresh}
          disabled={scanning}
        >
          Refresh
        </button>
      </div>

      {snapshot.status === "unavailable" ? (
        <div className="mt-4">
          <div className="alert alert-info">
            <div className="flex flex-col">
              <div className="font-bold">
                Service discovery: Local companion only
              </div>
              <div className="text-sm">
                The Web app can’t do mDNS/DNS-SD. Use IP scan (advanced) or
                connect by USB first.
              </div>
            </div>
          </div>
        </div>
      ) : null}

      {snapshot.error ? (
        <div className="mt-4">
          <div className="alert alert-warning">
            <div className="flex flex-col">
              <div className="font-bold">Discovery hint</div>
              <div className="text-sm">{snapshot.error}</div>
            </div>
          </div>
        </div>
      ) : null}

      <div className="mt-4">
        <input
          className="h-[40px] w-full rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-4 text-[13px] font-medium text-[var(--text)] outline-none placeholder:text-[var(--muted)]"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter by hostname / device_id"
          autoComplete="off"
        />
      </div>

      <div className="mt-4 min-h-0 flex-1 overflow-y-auto rounded-[14px] border border-[var(--border)] bg-[var(--panel-2)]">
        {filteredDevices.length === 0 ? (
          <div className="p-4 text-[12px] font-semibold text-[var(--muted)]">
            {emptyLabel}
          </div>
        ) : (
          <div className="divide-y divide-[var(--border)]">
            {filteredDevices.map((d) => {
              const added = isDiscoveredDeviceAdded(
                d,
                existingDeviceIds,
                existingDeviceBaseUrls,
              );
              return (
                <div
                  key={`${d.device_id ?? d.baseUrl}`}
                  className="flex items-start justify-between gap-4 px-4 py-3"
                >
                  <div className="min-w-0">
                    <div className="flex min-w-0 items-center gap-2">
                      <div className="min-w-0 truncate text-[13px] font-bold">
                        {unknown(d.fqdn ?? d.hostname)}
                      </div>
                      {added ? (
                        <div className="badge bg-[var(--badge-success-bg)] text-[var(--badge-success-text)]">
                          Added
                        </div>
                      ) : null}
                    </div>
                    <div className="mt-1 text-[12px] font-semibold text-[var(--muted)]">
                      ipv4: {unknown(d.ipv4)} • device_id:{" "}
                      {unknown(d.device_id)}
                    </div>
                    <div className="mt-1 min-w-0 truncate font-mono text-[12px] font-semibold text-[var(--muted)]">
                      baseUrl: {unknown(d.baseUrl)}
                    </div>
                    <div className="mt-1 text-[12px] font-semibold text-[var(--muted)]">
                      fw: {unknown(d.firmware?.version)}
                    </div>
                  </div>

                  <button
                    className="btn btn-sm"
                    type="button"
                    onClick={() => onSelect(d)}
                    disabled={added}
                  >
                    Add
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="mt-4 rounded-[14px] border border-[var(--border)] bg-[var(--panel)] px-4 py-3">
        {!ipScanExpanded ? (
          <div className="flex items-center justify-between">
            <div className="text-[13px] font-bold">IP scan (advanced)</div>
            <button
              className="link link-hover flex items-center gap-2 text-[12px] font-bold"
              type="button"
              onClick={() => onToggleIpScan(true)}
            >
              <span>Show</span>
              <Chevron direction="down" />
            </button>
          </div>
        ) : (
          <div>
            <div className="flex items-center justify-between">
              <div className="text-[13px] font-bold">IP scan (advanced)</div>
              <button
                className="link link-hover flex items-center gap-2 text-[12px] font-bold"
                type="button"
                onClick={() => onToggleIpScan(false)}
                disabled={scanning}
              >
                <span>Hide</span>
                <Chevron direction="up" />
              </button>
            </div>

            <div className="mt-3 flex items-center gap-2">
              <input
                className="h-[40px] flex-1 rounded-[12px] border border-[var(--border)] bg-[var(--panel-2)] px-4 text-[13px] font-medium text-[var(--text)] outline-none placeholder:text-[var(--muted)]"
                value={cidr}
                onChange={(e) => {
                  setCidrTouched(true);
                  setCidr(e.target.value);
                }}
                onKeyDown={(e) => {
                  if (e.key !== "Enter") {
                    return;
                  }
                  e.preventDefault();
                  if (scanning) {
                    return;
                  }
                  startScan();
                }}
                placeholder="CIDR, e.g. 192.168.1.0/24"
                autoComplete="off"
                list={
                  ipScanCandidatesList.length > 1
                    ? "ip-scan-candidates"
                    : undefined
                }
                disabled={scanning}
              />
              {ipScanCandidatesList.length > 1 ? (
                <datalist id="ip-scan-candidates">
                  {ipScanCandidatesList.map((c) => {
                    const meta = [
                      c.label ?? c.interface,
                      c.ipv4 ? `${c.cidr} · ${c.ipv4}` : c.cidr,
                    ]
                      .filter(Boolean)
                      .join(" / ");
                    return (
                      <option
                        key={`${c.interface ?? "if"}:${c.cidr}`}
                        value={c.cidr}
                      >
                        {meta}
                      </option>
                    );
                  })}
                </datalist>
              ) : null}
              <button
                className="btn btn-sm h-[40px]"
                type="button"
                onClick={startScan}
                disabled={scanning}
              >
                Scan
              </button>
            </div>

            {ipScanCandidates &&
            ipScanCandidates.length === 0 &&
            cidr.trim().length === 0 ? (
              <div className="mt-2 text-[12px] font-semibold text-[var(--muted)]">
                No local network candidates. Enter a CIDR range.
              </div>
            ) : null}

            {cidrError ? (
              <div className="mt-2 text-[12px] font-semibold text-[var(--error)]">
                {cidrError}
              </div>
            ) : null}

            {scanning && snapshot.scan ? (
              <div className="mt-2 flex items-center justify-between text-[12px] font-semibold text-[var(--muted)]">
                <div>
                  {snapshot.scan.done}/{snapshot.scan.total} probed
                </div>
                <button
                  className="link link-hover text-[12px] font-bold"
                  type="button"
                  onClick={onCancelScan}
                >
                  Cancel
                </button>
              </div>
            ) : null}
          </div>
        )}
      </div>
    </div>
  );
}
