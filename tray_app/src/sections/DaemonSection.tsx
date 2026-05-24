import React from "react";
import { Copy, Check, Play, Square, RotateCw, ScrollText } from "lucide-react";
import SectionShell from "@/components/SectionShell";
import StatusBadge from "@/components/StatusBadge";
import { useCopy } from "@/hooks/useCopy";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { DaemonInfo } from "@/api/pirouter";

interface DaemonSectionProps {
  daemon?: DaemonInfo;
  loading?: boolean;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onOpenLogs: () => void;
}

const DaemonSection: React.FC<DaemonSectionProps> = ({
  daemon,
  loading = false,
  onStart,
  onStop,
  onRestart,
  onOpenLogs,
}) => {
  const endpointCopy = useCopy();
  const status = daemon?.status ?? (loading ? "stopped" : "error");
  const endpoint = daemon?.endpoint ?? "http://127.0.0.1:11435/v1";
  const configPath = daemon?.configPath ?? "Config unavailable";
  const ledgerPath = daemon?.ledgerPath ?? "Ledger unavailable";

  const isRunning = status === "running";
  const statusVariant = isRunning ? "running" : status === "error" ? "error" : "stopped";
  const statusLabel = loading ? "Loading" : status.charAt(0).toUpperCase() + status.slice(1);

  return (
    <SectionShell
      title="Daemon"
      action={
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onOpenLogs}
                className="h-8 px-3 rounded-button text-button text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted transition-colors duration-120 flex items-center gap-1.5"
              >
                <ScrollText className="w-4 h-4" />
                <span>Logs</span>
              </button>
            </TooltipTrigger>
            <TooltipContent>Open daemon logs</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      }
    >
      {/* Field grid: 2 columns */}
      <div className="grid grid-cols-2 gap-x-4 gap-y-field-gap">
        {/* Row 1: Status + Mode */}
        <div className="flex items-center gap-2">
          <span className="text-label text-pirouter-text-muted w-[72px] shrink-0">Status</span>
          <StatusBadge variant={statusVariant}>{statusLabel}</StatusBadge>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-label text-pirouter-text-muted w-[72px] shrink-0">Mode</span>
          <span className="text-body text-pirouter-text">{daemon?.mode ?? "Desktop daemon"}</span>
        </div>
        {/* Row 2: Endpoint + Config */}
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-label text-pirouter-text-muted w-[72px] shrink-0">Endpoint</span>
          <span className="text-body text-pirouter-text font-mono truncate">
            {endpoint}
          </span>
        </div>
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-label text-pirouter-text-muted w-[72px] shrink-0">Config</span>
          <span className="text-metadata text-pirouter-text font-mono truncate">
            {configPath}
          </span>
        </div>
        {/* Row 3: Bind + Ledger */}
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-label text-pirouter-text-muted w-[72px] shrink-0">Bind</span>
          <span className="text-body text-pirouter-text font-mono truncate">
            {daemon?.bind ?? "127.0.0.1:11435"}
          </span>
        </div>
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-label text-pirouter-text-muted w-[72px] shrink-0">Ledger</span>
          <span className="text-metadata text-pirouter-text font-mono truncate">
            {ledgerPath}
          </span>
        </div>
      </div>

      {/* Action buttons */}
      <div className="flex gap-2 mt-1">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onStart}
                disabled={isRunning || loading}
                className="h-8 px-4 rounded-button text-button inline-flex items-center gap-2 bg-pirouter-success text-white hover:bg-[#147a3f] disabled:opacity-40 disabled:cursor-not-allowed transition-colors duration-120"
              >
                <Play className="w-4 h-4" />
                Start
              </button>
            </TooltipTrigger>
            <TooltipContent>Start daemon</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onStop}
                disabled={!isRunning || loading}
                className="h-8 px-4 rounded-button text-button inline-flex items-center gap-2 bg-pirouter-surface-muted text-pirouter-text hover:bg-pirouter-border disabled:opacity-40 disabled:cursor-not-allowed transition-colors duration-120 border border-pirouter-border"
              >
                <Square className="w-4 h-4" />
                Stop
              </button>
            </TooltipTrigger>
            <TooltipContent>Stop daemon</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onRestart}
                disabled={loading}
                className="h-8 px-4 rounded-button text-button inline-flex items-center gap-2 bg-pirouter-surface-muted text-pirouter-text hover:bg-pirouter-border disabled:opacity-40 disabled:cursor-not-allowed transition-colors duration-120 border border-pirouter-border"
              >
                <RotateCw className="w-4 h-4" />
                Restart
              </button>
            </TooltipTrigger>
            <TooltipContent>Restart daemon</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => endpointCopy.copy(endpoint)}
                className="h-8 px-4 rounded-button text-button inline-flex items-center gap-2 bg-pirouter-surface-muted text-pirouter-text hover:bg-pirouter-border transition-colors duration-120 border border-pirouter-border"
              >
                {endpointCopy.copied ? <Check className="w-4 h-4 text-pirouter-success" /> : <Copy className="w-4 h-4" />}
                Copy endpoint
              </button>
            </TooltipTrigger>
            <TooltipContent>{endpointCopy.copied ? "Copied" : "Copy endpoint"}</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
    </SectionShell>
  );
};

export default DaemonSection;
