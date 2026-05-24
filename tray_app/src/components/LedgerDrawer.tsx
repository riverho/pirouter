import React, { useState, useEffect, useCallback } from "react";
import { X, RefreshCw, Download, ChevronDown, ChevronUp, Search } from "lucide-react";
import StatusBadge from "@/components/StatusBadge";
import { useCopy } from "@/hooks/useCopy";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { LedgerRow, LedgerStatus } from "@/api/pirouter";

/* ── status helper ── */
const ledgerStatusBadge = (status: LedgerStatus) => {
  const map = {
    ok: "success" as const,
    error: "error" as const,
    escalated: "warning" as const,
  };
  const labels = { ok: "OK", error: "ERR", escalated: "ESC" };
  return <StatusBadge variant={map[status]}>{labels[status]}</StatusBadge>;
};

/* ── component ── */
interface LedgerDrawerProps {
  open: boolean;
  rows: LedgerRow[];
  endpoint: string;
  onRefresh: () => void | Promise<void>;
  onClose: () => void;
}

const PAGE_SIZE = 8;

const LedgerDrawer: React.FC<LedgerDrawerProps> = ({ open, rows, endpoint, onRefresh, onClose }) => {
  const [timeRange, setTimeRange] = useState<"1h" | "24h" | "7d" | "custom">("24h");
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(false);
  const endpointCopy = useCopy();

  /* Esc to close */
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    },
    [onClose]
  );

  useEffect(() => {
    if (open) {
      window.addEventListener("keydown", handleKeyDown);
      document.body.style.overflow = "hidden";
    }
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = "";
    };
  }, [open, handleKeyDown]);

  /* Filter */
  const filtered = rows.filter((row) => {
    if (statusFilter !== "all" && row.status !== statusFilter) return false;
    if (search) {
      const q = search.toLowerCase();
      return (
        row.requested.toLowerCase().includes(q) ||
        row.finalModel.toLowerCase().includes(q) ||
        row.rule.toLowerCase().includes(q)
      );
    }
    return true;
  });

  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const pageStart = page * PAGE_SIZE;
  const pageEnd = Math.min(pageStart + PAGE_SIZE, filtered.length);
  const visible = filtered.slice(pageStart, pageEnd);

  const totalCost = filtered.reduce((s, r) => s + r.cost, 0);
  const avgLat = filtered.length ? Math.round(filtered.reduce((s, r) => s + r.latency, 0) / filtered.length) : 0;

  /* Refresh */
  const handleRefresh = async () => {
    setLoading(true);
    try {
      await onRefresh();
    } finally {
      setLoading(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[60] flex justify-end">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/20 transition-opacity duration-140"
        onClick={onClose}
      />

      {/* Drawer panel */}
      <div className="relative w-[860px] min-w-[760px] max-w-[calc(100vw-48px)] h-full bg-pirouter-surface border-l border-pirouter-border flex flex-col shadow-lg animate-in slide-in-from-right duration-140">
        {/* ── Header ── */}
        <div className="shrink-0 h-[72px] px-5 flex flex-col justify-center gap-0.5 border-b border-pirouter-border">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <h2 className="text-page-title text-pirouter-text font-semibold">Ledger</h2>
            </div>
            <div className="flex items-center gap-2">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      onClick={handleRefresh}
                      className="h-8 w-8 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted border border-pirouter-border transition-colors duration-120"
                    >
                      <RefreshCw className={`w-4 h-4 ${loading ? "animate-spin" : ""}`} />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent>Refresh</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      disabled={filtered.length === 0}
                      className="h-8 px-3 rounded-button text-button inline-flex items-center gap-1.5 bg-pirouter-surface-muted text-pirouter-text hover:bg-pirouter-border disabled:opacity-40 disabled:cursor-not-allowed border border-pirouter-border transition-colors duration-120"
                    >
                      <Download className="w-4 h-4" />
                      Export CSV
                    </button>
                  </TooltipTrigger>
                  <TooltipContent>Export filtered rows</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      onClick={onClose}
                      className="h-8 w-8 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted border border-pirouter-border transition-colors duration-120"
                    >
                      <X className="w-4 h-4" />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent>Close (Esc)</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>
          {/* Summary row */}
          <div className="flex items-center gap-1 text-metadata text-pirouter-text-muted">
            <span>Last 24 hours</span>
            <span>·</span>
            <span>{filtered.length} requests</span>
            <span>·</span>
            <span>${totalCost.toFixed(4)}</span>
            <span>·</span>
            <span>avg {avgLat}ms</span>
          </div>
        </div>

        {/* ── Filter bar ── */}
        <div className="shrink-0 h-14 px-5 flex items-center gap-2 border-b border-pirouter-border bg-pirouter-surface">
          {/* Time range */}
          <div className="flex rounded-input bg-pirouter-surface-muted p-0.5 h-8">
            {(["1h", "24h", "7d", "custom"] as const).map((r) => (
              <button
                key={r}
                onClick={() => { setTimeRange(r); setPage(0); }}
                className={`h-7 px-3 rounded-[5px] text-button transition-colors duration-120 ${
                  timeRange === r ? "bg-pirouter-surface text-pirouter-text shadow-xs" : "text-pirouter-text-muted hover:text-pirouter-text"
                }`}
              >
                {r}
              </button>
            ))}
          </div>
          {/* Status select */}
          <select
            value={statusFilter}
            onChange={(e) => { setStatusFilter(e.target.value); setPage(0); }}
            className="h-8 px-2 rounded-input border border-pirouter-border bg-pirouter-surface text-metadata text-pirouter-text focus:outline-none focus:ring-2 focus:ring-pirouter-accent/30 w-28"
          >
            <option value="all">All status</option>
            <option value="ok">OK</option>
            <option value="escalated">Escalated</option>
            <option value="error">Error</option>
          </select>
          {/* Search */}
          <div className="relative flex-1 min-w-[180px]">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-pirouter-text-muted" />
            <input
              type="text"
              value={search}
              onChange={(e) => { setSearch(e.target.value); setPage(0); }}
              placeholder="Search model, rule..."
              className="h-8 w-full pl-8 pr-3 rounded-input border border-pirouter-border bg-pirouter-surface text-metadata text-pirouter-text placeholder:text-pirouter-text-muted focus:outline-none focus:ring-2 focus:ring-pirouter-accent/30 focus:border-pirouter-accent transition-all duration-120"
            />
          </div>
        </div>

        {/* ── Table ── */}
        <div className="flex-1 overflow-y-auto px-5">
          {loading ? (
            /* Skeleton */
            <div className="flex flex-col">
              {Array.from({ length: 6 }).map((_, i) => (
                <div key={i} className="h-11 flex items-center gap-2 animate-pulse">
                  <div className="w-24 h-4 bg-pirouter-surface-muted rounded" />
                  <div className="flex-1 h-4 bg-pirouter-surface-muted rounded" />
                  <div className="w-16 h-4 bg-pirouter-surface-muted rounded" />
                  <div className="w-12 h-4 bg-pirouter-surface-muted rounded" />
                </div>
              ))}
            </div>
          ) : filtered.length === 0 ? (
            /* Empty state */
            <div className="flex flex-col items-center justify-center" style={{ height: "240px" }}>
              <div className="w-[420px] text-center">
                <p className="text-body text-pirouter-text font-semibold mb-1">No ledger rows in this range</p>
                <p className="text-metadata text-pirouter-text-muted mb-4">Run an OpenAI-compatible request through pirouter, then refresh.</p>
                <button
                  onClick={() => endpointCopy.copy(endpoint)}
                  className="h-8 px-4 rounded-button text-button bg-pirouter-accent text-white hover:bg-[#0d8b7d] transition-colors duration-120"
                >
                  {endpointCopy.copied ? "Copied" : "Copy endpoint"}
                </button>
              </div>
            </div>
          ) : (
            /* Table */
            <table className="w-full">
              <thead className="sticky top-0 bg-pirouter-surface z-10">
                <tr className="h-9 border-b border-pirouter-border">
                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-[112px]">Time</th>
                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-[104px]">Requested</th>
                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-[104px]">Final</th>
                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-[132px]">Rule</th>
                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-[76px]">Status</th>
                  <th className="text-table-header text-pirouter-text-muted uppercase text-right w-[68px]">Input</th>
                  <th className="text-table-header text-pirouter-text-muted uppercase text-right w-[68px]">Output</th>
                  <th className="text-table-header text-pirouter-text-muted uppercase text-right w-[72px]">Cost</th>
                  <th className="text-table-header text-pirouter-text-muted uppercase text-right w-[76px]">Latency</th>
                  <th className="text-table-header text-pirouter-text-muted uppercase text-right w-10"></th>
                </tr>
              </thead>
              <tbody>
                {visible.map((row) => (
                  <React.Fragment key={row.id}>
                    <tr
                      className="h-11 border-b border-pirouter-border/50 hover:bg-pirouter-surface-muted/40 transition-colors duration-120 cursor-pointer"
                      onClick={() => setExpandedId(expandedId === row.id ? null : row.id)}
                    >
                      <td className="text-metadata text-pirouter-text-muted tabular-nums">{row.ts}</td>
                      <td className="text-body text-pirouter-text truncate" title={row.requested}>{row.requested}</td>
                      <td className="text-body text-pirouter-text truncate" title={row.finalModel}>{row.finalModel}</td>
                      <td className="text-metadata text-pirouter-text-muted truncate font-mono" title={row.rule}>{row.rule}</td>
                      <td>{ledgerStatusBadge(row.status)}</td>
                      <td className="text-metadata text-pirouter-text-muted text-right tabular-nums">{row.inputTokens}</td>
                      <td className="text-metadata text-pirouter-text-muted text-right tabular-nums">{row.outputTokens}</td>
                      <td className="text-metadata text-pirouter-text-muted text-right tabular-nums">${row.cost.toFixed(4)}</td>
                      <td className="text-metadata text-pirouter-text-muted text-right tabular-nums">{row.latency}ms</td>
                      <td className="text-right">
                        <button
                          className="h-8 w-8 rounded-button inline-flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted transition-colors duration-120"
                          onClick={(e) => { e.stopPropagation(); setExpandedId(expandedId === row.id ? null : row.id); }}
                        >
                          {expandedId === row.id ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
                        </button>
                      </td>
                    </tr>
                    {/* Expanded detail */}
                    {expandedId === row.id && (
                      <tr>
                        <td colSpan={10} className="bg-pirouter-bg border-y border-pirouter-border p-3">
                          {/* Request summary */}
                          <div className="mb-3">
                            <p className="text-label text-pirouter-text-muted uppercase mb-1">Request</p>
                            <div className="flex gap-4 text-metadata text-pirouter-text">
                              <span>requested: <span className="font-mono">{row.requested}</span></span>
                              <span>final: <span className="font-mono">{row.finalModel}</span></span>
                              <span>status: {row.status}</span>
                            </div>
                          </div>
                          {/* Cascade table */}
                          <div>
                            <p className="text-label text-pirouter-text-muted uppercase mb-1">Cascade</p>
                            <table className="w-full">
                              <thead>
                                <tr className="h-7 border-b border-pirouter-border">
                                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-14">Attempt</th>
                                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-32">Alias</th>
                                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-[88px]">Provider</th>
                                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-[260px]">Model ID</th>
                                  <th className="text-table-header text-pirouter-text-muted uppercase text-right w-[72px]">Latency</th>
                                  <th className="text-table-header text-pirouter-text-muted uppercase text-left w-[120px]">Outcome</th>
                                </tr>
                              </thead>
                              <tbody>
                                {row.cascade.map((ca) => (
                                  <tr key={ca.attempt} className="h-8">
                                    <td className="text-metadata text-pirouter-text-muted tabular-nums">{ca.attempt}</td>
                                    <td className="text-body text-pirouter-text font-medium">{ca.alias}</td>
                                    <td className="text-metadata text-pirouter-text-muted">{ca.provider}</td>
                                    <td className="text-metadata text-pirouter-text-muted truncate font-mono">{ca.modelId}</td>
                                    <td className="text-metadata text-pirouter-text-muted text-right tabular-nums">{ca.latency}ms</td>
                                    <td className="text-metadata">
                                      <span className={`${ca.outcome === "ok" ? "text-pirouter-success" : "text-pirouter-warning"}`}>
                                        {ca.outcome}
                                      </span>
                                    </td>
                                  </tr>
                                ))}
                              </tbody>
                            </table>
                          </div>
                        </td>
                      </tr>
                    )}
                  </React.Fragment>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* ── Footer ── */}
        <div className="shrink-0 h-14 px-5 flex items-center justify-between border-t border-pirouter-border">
          <span className="text-metadata text-pirouter-text-muted">
            {filtered.length > 0 ? `${pageStart + 1}-${pageEnd} of ${filtered.length}` : "0 rows"}
          </span>
          <div className="flex items-center gap-1">
            <button
              onClick={() => setPage((p) => Math.max(0, p - 1))}
              disabled={page === 0}
              className="h-8 px-3 rounded-button text-button text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted disabled:opacity-40 disabled:cursor-not-allowed border border-pirouter-border transition-colors duration-120"
            >
              Prev
            </button>
            <button
              onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
              disabled={page >= totalPages - 1}
              className="h-8 px-3 rounded-button text-button text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted disabled:opacity-40 disabled:cursor-not-allowed border border-pirouter-border transition-colors duration-120"
            >
              Next
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default LedgerDrawer;
