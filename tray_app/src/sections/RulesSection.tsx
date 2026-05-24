import React from "react";
import { FileCode2 } from "lucide-react";
import SectionShell from "@/components/SectionShell";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { RuleInfo } from "@/api/pirouter";

interface RulesSectionProps {
  rules: RuleInfo[];
}

const RulesSection: React.FC<RulesSectionProps> = ({ rules }) => {
  return (
    <SectionShell
      title="Rules"
      action={
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <button className="h-8 px-3 rounded-button text-button text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted transition-colors duration-120 flex items-center gap-1.5">
                <FileCode2 className="w-4 h-4" />
                <span>Edit TOML</span>
              </button>
            </TooltipTrigger>
            <TooltipContent>Edit rules in config file</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      }
    >
      {/* Header */}
      <div className="grid grid-cols-[120px_1fr_96px] gap-2 items-center h-7">
        <span className="text-table-header text-pirouter-text-muted uppercase">Name</span>
        <span className="text-table-header text-pirouter-text-muted uppercase">Predicate</span>
        <span className="text-table-header text-pirouter-text-muted uppercase">Target</span>
      </div>

      {/* Rule rows */}
      <div className="flex flex-col gap-0">
        {rules.length === 0 && (
          <div className="h-12 flex items-center text-metadata text-pirouter-text-muted">
            No explicit rules; policy routing will choose models.
          </div>
        )}
        {rules.map((rule) => (
          <div
            key={rule.id}
            className="grid grid-cols-[120px_1fr_96px] gap-2 items-center h-9 hover:bg-pirouter-surface-muted/40 transition-colors duration-120 rounded-sm px-1 -mx-1"
          >
            <span className="text-body text-pirouter-text font-medium truncate">{rule.name}</span>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="text-metadata text-pirouter-text-muted truncate font-mono">{rule.predicate}</span>
                </TooltipTrigger>
                <TooltipContent>{rule.predicate}</TooltipContent>
              </Tooltip>
            </TooltipProvider>
            <span className="text-metadata text-pirouter-text truncate">{rule.target}</span>
          </div>
        ))}
      </div>
    </SectionShell>
  );
};

export default RulesSection;
