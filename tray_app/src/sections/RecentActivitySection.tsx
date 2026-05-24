import React from "react";
import { BookOpen } from "lucide-react";
import SectionShell from "@/components/SectionShell";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { LedgerRow } from "@/api/pirouter";

const statusBadge = (status: LedgerRow["status"]) => {
  const map = {
    ok: { bg: "bg-pirouter-success-soft", text: "text-pirouter-success", label: "OK" },
    error: { bg: "bg-pirouter-danger-soft", text: "text-pirouter-danger", label: "ERR" },
    escalated: { bg: "bg-pirouter-warning-soft", text: "text-pirouter-warning", label: "ESC" },
  };
  const s = map[status];
  return (
    <span className={`inline-flex items-center h-5 px-2 rounded-badge text-metadata font-semibold ${s.bg} ${s.text}`}>
      {s.label}
    </span>
  );
};

interface RecentActivitySectionProps {
  rows: LedgerRow[];
  onOpenLedger: () => void;
}

const RecentActivitySection: React.FC<RecentActivitySectionProps> = ({ rows, onOpenLedger }) => {
  return (
    <SectionShell
      title="Recent Activity"
      action={
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onOpenLedger}
                className="h-8 px-3 rounded-button text-button bg-pirouter-accent text-white hover:bg-[#0d8b7d] transition-colors duration-120 flex items-center gap-1.5"
              >
                <BookOpen className="w-4 h-4" />
                <span>Open Ledger</span>
              </button>
            </TooltipTrigger>
            <TooltipContent>View full routing ledger</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      }
    >
      {/* Header */}
      <div className="grid grid-cols-[72px_1fr_64px_64px_56px] gap-2 items-center h-7">
        <span className="text-table-header text-pirouter-text-muted uppercase">Time</span>
        <span className="text-table-header text-pirouter-text-muted uppercase">Final</span>
        <span className="text-table-header text-pirouter-text-muted uppercase text-right">Cost</span>
        <span className="text-table-header text-pirouter-text-muted uppercase text-right">Lat</span>
        <span className="text-table-header text-pirouter-text-muted uppercase text-right">Sts</span>
      </div>

      {/* Activity rows */}
      <div className="flex flex-col gap-0">
        {rows.length === 0 && (
          <div className="h-16 flex items-center text-metadata text-pirouter-text-muted">
            No requests recorded yet.
          </div>
        )}
        {rows.map((row) => (
          <div
            key={row.id}
            className="grid grid-cols-[72px_1fr_64px_64px_56px] gap-2 items-center h-10 hover:bg-pirouter-surface-muted/40 transition-colors duration-120 rounded-sm px-1 -mx-1"
          >
            <span className="text-metadata text-pirouter-text-muted tabular-nums">{row.ts}</span>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="text-body text-pirouter-text truncate">{row.finalModel}</span>
                </TooltipTrigger>
                <TooltipContent>Requested: {row.requested} - Routed: {row.finalModel}</TooltipContent>
              </Tooltip>
            </TooltipProvider>
            <span className="text-metadata text-pirouter-text-muted text-right tabular-nums">${row.cost.toFixed(4)}</span>
            <span className="text-metadata text-pirouter-text-muted text-right tabular-nums">{row.latency}ms</span>
            <div className="flex justify-end">{statusBadge(row.status)}</div>
          </div>
        ))}
      </div>
    </SectionShell>
  );
};

export default RecentActivitySection;
