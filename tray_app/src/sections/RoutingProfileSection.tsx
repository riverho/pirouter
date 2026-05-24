import React from "react";
import { RotateCcw } from "lucide-react";
import SectionShell from "@/components/SectionShell";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { RoutingInfo } from "@/api/pirouter";

const profiles = ["local-first", "balanced", "best-quality", "cloud-only"] as const;

interface RoutingProfileSectionProps {
  routing?: RoutingInfo;
}

const RoutingProfileSection: React.FC<RoutingProfileSectionProps> = ({ routing }) => {
  const activeProfile = routing?.profile ?? "balanced";
  const autoCascade = routing?.autoCascade ?? true;
  const maxFallbacks = routing?.maxFallbacks ?? 3;
  const shortResponse = routing?.onShortResponse ?? false;
  const minTokens = String(routing?.minOutputTokens ?? 8);
  const marker = routing?.marker ?? "";

  return (
    <SectionShell
      title="Routing Profile"
      action={
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <button className="h-8 w-8 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted transition-colors duration-120">
                <RotateCcw className="w-4 h-4" />
              </button>
            </TooltipTrigger>
            <TooltipContent>Reset to defaults</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      }
    >
      {/* Profile segmented control */}
      <div className="flex rounded-input bg-pirouter-surface-muted p-0.5">
        {profiles.map((p) => (
          <button
            key={p}
            disabled
            className={`flex-1 h-8 rounded-[5px] text-button transition-colors duration-120 ${
              activeProfile === p
                ? "bg-pirouter-surface text-pirouter-text shadow-xs"
                : "text-pirouter-text-muted hover:text-pirouter-text"
            }`}
          >
            {p}
          </button>
        ))}
      </div>

      {/* Settings rows */}
      <div className="flex flex-col gap-field-gap">
        {/* Auto cascade */}
        <div className="flex items-center justify-between h-8">
          <span className="text-body text-pirouter-text">Auto cascade</span>
          <Switch checked={autoCascade} disabled className="data-[state=checked]:bg-pirouter-accent" />
        </div>

        {/* Max fallbacks */}
        <div className="flex items-center justify-between h-8">
          <span className="text-body text-pirouter-text">Max fallbacks</span>
          <div className="flex items-center">
            <button
              disabled
              className="h-8 w-8 rounded-l-button bg-pirouter-surface-muted border border-pirouter-border border-r-0 flex items-center justify-center text-pirouter-text hover:bg-pirouter-border transition-colors duration-120"
            >
              −
            </button>
            <span className="h-8 w-10 flex items-center justify-center border border-pirouter-border text-body text-pirouter-text tabular-nums bg-pirouter-surface">
              {maxFallbacks}
            </span>
            <button
              disabled
              className="h-8 w-8 rounded-r-button bg-pirouter-surface-muted border border-pirouter-border border-l-0 flex items-center justify-center text-pirouter-text hover:bg-pirouter-border transition-colors duration-120"
            >
              +
            </button>
          </div>
        </div>

        {/* Short response */}
        <div className="flex items-center justify-between h-8">
          <span className="text-body text-pirouter-text">Short response</span>
          <Switch checked={shortResponse} disabled className="data-[state=checked]:bg-pirouter-accent" />
        </div>

        {/* Min output tokens */}
        <div className="flex items-center justify-between h-8">
          <span className="text-body text-pirouter-text">Min output tokens</span>
          <input
            type="text"
            value={minTokens}
            readOnly
            className="h-8 w-16 px-2 rounded-input border border-pirouter-border bg-pirouter-surface text-body text-pirouter-text text-right tabular-nums focus:outline-none focus:ring-2 focus:ring-pirouter-accent/30 focus:border-pirouter-accent transition-all duration-120"
          />
        </div>

        {/* Marker */}
        <div className="flex items-center justify-between h-8">
          <span className="text-body text-pirouter-text">Marker</span>
          <input
            type="text"
            value={marker || "-"}
            readOnly
            className="h-8 w-36 px-2 rounded-input border border-pirouter-border bg-pirouter-surface text-body text-pirouter-text font-mono focus:outline-none focus:ring-2 focus:ring-pirouter-accent/30 focus:border-pirouter-accent transition-all duration-120"
          />
        </div>
      </div>
    </SectionShell>
  );
};

export default RoutingProfileSection;
