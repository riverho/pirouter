import React from "react";
import { Copy, Check, ScrollText, Square, RotateCw, Command, Power } from "lucide-react";
import StatusBadge from "@/components/StatusBadge";
import { useCopy } from "@/hooks/useCopy";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { DaemonStatus } from "@/api/pirouter";

interface TopHeaderProps {
  daemonStatus?: DaemonStatus;
  currentProfile?: string;
  endpoint?: string;
  onStop?: () => void;
  onRestart?: () => void;
  onOpenLogs?: () => void;
  onQuit?: () => void;
}

const TopHeader: React.FC<TopHeaderProps> = ({
  daemonStatus = "running",
  currentProfile = "balanced",
  endpoint = "http://127.0.0.1:11435/v1",
  onStop,
  onRestart,
  onOpenLogs,
  onQuit,
}) => {
  const endpointCopy = useCopy();

  const statusVariant = daemonStatus === "running" ? "running" : daemonStatus === "error" ? "error" : daemonStatus === "restart-required" ? "warning" : "stopped";
  const statusLabel = daemonStatus === "restart-required" ? "Restart required" : daemonStatus.charAt(0).toUpperCase() + daemonStatus.slice(1);

  return (
    <div className="w-full h-[72px] bg-pirouter-surface border border-pirouter-border rounded-section shadow-section flex items-center px-4">
      {/* Left group */}
      <div className="flex-1 flex flex-col justify-center gap-0.5 min-w-0">
        <div className="flex items-center gap-3 h-7">
          {/* Text logo */}
          <h1 className="text-page-title text-pirouter-text font-semibold tracking-normal select-none">
            pirouter
          </h1>
          <StatusBadge variant={statusVariant}>{statusLabel}</StatusBadge>
        </div>
        <div className="flex items-center gap-2 h-5">
          <span className="text-metadata text-pirouter-text-muted">Smart model router</span>
          <span className="text-metadata text-pirouter-border">·</span>
          <span className="text-metadata text-pirouter-text-muted">{currentProfile} profile</span>
        </div>
      </div>

      {/* Right group */}
      <div className="flex items-center gap-2">
        {/* Endpoint pill */}
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => endpointCopy.copy(endpoint)}
                className="h-8 min-w-[260px] max-w-[340px] px-2.5 rounded-input bg-pirouter-surface-muted border border-pirouter-border flex items-center gap-2 text-metadata text-pirouter-text font-mono hover:border-pirouter-accent/40 hover:bg-pirouter-accent-soft/30 transition-all duration-120 group"
              >
                <span className="truncate">{endpoint}</span>
                {endpointCopy.copied ? (
                  <Check className="w-3.5 h-3.5 text-pirouter-success shrink-0" />
                ) : (
                  <Copy className="w-3.5 h-3.5 text-pirouter-text-muted group-hover:text-pirouter-link shrink-0" />
                )}
              </button>
            </TooltipTrigger>
            <TooltipContent>{endpointCopy.copied ? "Copied!" : "Copy endpoint"}</TooltipContent>
          </Tooltip>

          {/* Action buttons */}
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onOpenLogs}
                aria-label="Open logs"
                className="h-8 w-8 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted border border-pirouter-border transition-colors duration-120"
              >
                <ScrollText className="w-4 h-4" />
              </button>
            </TooltipTrigger>
            <TooltipContent>Open logs</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                aria-label="Command palette"
                className="h-8 w-8 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted border border-pirouter-border transition-colors duration-120"
              >
                <Command className="w-4 h-4" />
              </button>
            </TooltipTrigger>
            <TooltipContent>Command palette</TooltipContent>
          </Tooltip>

          <div className="w-px h-5 bg-pirouter-border mx-1" />

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onStop}
                aria-label="Stop daemon"
                className="h-8 w-8 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-danger hover:bg-pirouter-danger-soft border border-pirouter-border transition-colors duration-120"
              >
                <Square className="w-4 h-4" />
              </button>
            </TooltipTrigger>
            <TooltipContent>Stop daemon</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onRestart}
                aria-label="Restart daemon"
                className="h-8 w-8 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted border border-pirouter-border transition-colors duration-120"
              >
                <RotateCw className="w-4 h-4" />
              </button>
            </TooltipTrigger>
            <TooltipContent>Restart daemon</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onQuit}
                aria-label="Quit app"
                className="h-8 w-8 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-danger hover:bg-pirouter-danger-soft border border-pirouter-border transition-colors duration-120"
              >
                <Power className="w-4 h-4" />
              </button>
            </TooltipTrigger>
            <TooltipContent>Quit app</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
    </div>
  );
};

export default TopHeader;
