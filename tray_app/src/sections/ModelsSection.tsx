import React from "react";
import { Pencil, MoreHorizontal, Cpu } from "lucide-react";
import SectionShell from "@/components/SectionShell";
import { Switch } from "@/components/ui/switch";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { ModelInfo } from "@/api/pirouter";

interface ModelsSectionProps {
  models: ModelInfo[];
}

const qualityBadge = (quality: string) => {
  const map = {
    basic: { bg: "bg-pirouter-surface-muted", text: "text-pirouter-text-muted" },
    standard: { bg: "bg-pirouter-accent-soft", text: "text-pirouter-accent" },
    strong: { bg: "bg-pirouter-warning-soft", text: "text-pirouter-warning" },
    premium: { bg: "bg-pirouter-warning-soft", text: "text-pirouter-warning" },
  };
  const q = map[quality as keyof typeof map] ?? map.standard;
  return (
    <span className={`inline-flex items-center h-5 px-2 rounded-badge text-metadata font-semibold ${q.bg} ${q.text}`}>
      {quality}
    </span>
  );
};

const ModelsSection: React.FC<ModelsSectionProps> = ({ models }) => {
  return (
    <SectionShell
      title="Models"
      action={
        <div className="flex items-center gap-1">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <button className="h-8 px-3 rounded-button text-button text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted transition-colors duration-120 flex items-center gap-1.5">
                  <Cpu className="w-4 h-4" />
                  <span>Import</span>
                </button>
              </TooltipTrigger>
              <TooltipContent>Import from Ollama</TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <button className="h-8 px-3 rounded-button text-button bg-pirouter-accent text-white hover:bg-[#0d8b7d] transition-colors duration-120 flex items-center gap-1.5">
            + Add
          </button>
        </div>
      }
    >
      {/* Table header */}
      <div className="grid grid-cols-[48px_96px_80px_1fr_80px_64px_44px_44px_72px_56px] gap-2 items-center h-8 border-b border-pirouter-border">
        <span className="text-table-header text-pirouter-text-muted uppercase">On</span>
        <span className="text-table-header text-pirouter-text-muted uppercase">Alias</span>
        <span className="text-table-header text-pirouter-text-muted uppercase">Provider</span>
        <span className="text-table-header text-pirouter-text-muted uppercase">Model ID</span>
        <span className="text-table-header text-pirouter-text-muted uppercase">Quality</span>
        <span className="text-table-header text-pirouter-text-muted uppercase text-right">Ctx</span>
        <span className="text-table-header text-pirouter-text-muted uppercase text-center">Tool</span>
        <span className="text-table-header text-pirouter-text-muted uppercase text-center">Vis</span>
        <span className="text-table-header text-pirouter-text-muted uppercase text-right">Cost</span>
        <span className="text-table-header text-pirouter-text-muted uppercase text-right">Act</span>
      </div>

      {/* Model rows */}
      <div className="flex flex-col gap-0 max-h-[220px] overflow-y-auto">
        {models.map((model) => (
          <div
            key={model.id}
            className="grid grid-cols-[48px_96px_80px_1fr_80px_64px_44px_44px_72px_56px] gap-2 items-center h-11 hover:bg-pirouter-surface-muted/40 transition-colors duration-120"
          >
            <Switch
              checked={model.enabled}
              disabled
              className="data-[state=checked]:bg-pirouter-accent scale-90 origin-left"
            />
            <span className="text-body text-pirouter-text font-semibold truncate">{model.alias}</span>
            <span className="text-metadata text-pirouter-text-muted truncate">{model.provider}</span>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="text-metadata text-pirouter-text-muted truncate font-mono">{model.modelId}</span>
                </TooltipTrigger>
                <TooltipContent>{model.modelId}</TooltipContent>
              </Tooltip>
            </TooltipProvider>
            <div>{qualityBadge(model.quality)}</div>
            <span className="text-metadata text-pirouter-text-muted text-right tabular-nums">{model.ctx}k</span>
            <span className={`text-metadata text-center ${model.tools ? "text-pirouter-success" : "text-pirouter-text-muted/30"}`}>
              {model.tools ? "✓" : "—"}
            </span>
            <span className={`text-metadata text-center ${model.vision ? "text-pirouter-success" : "text-pirouter-text-muted/30"}`}>
              {model.vision ? "✓" : "—"}
            </span>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="text-metadata text-pirouter-text-muted text-right tabular-nums truncate">
                    ${model.costIn}/${model.costOut}
                  </span>
                </TooltipTrigger>
                <TooltipContent>${model.costIn} in / ${model.costOut} out per 1M tokens</TooltipContent>
              </Tooltip>
            </TooltipProvider>
            <div className="flex items-center justify-end gap-0.5">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button className="h-7 w-7 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted transition-colors duration-120">
                      <Pencil className="w-3.5 h-3.5" />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent>Edit model</TooltipContent>
                </Tooltip>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <button className="h-7 w-7 rounded-button flex items-center justify-center text-pirouter-text-muted hover:text-pirouter-text hover:bg-pirouter-surface-muted transition-colors duration-120">
                      <MoreHorizontal className="w-3.5 h-3.5" />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem>Duplicate</DropdownMenuItem>
                    <DropdownMenuItem className="text-pirouter-danger">Delete</DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </TooltipProvider>
            </div>
          </div>
        ))}
      </div>
    </SectionShell>
  );
};

export default ModelsSection;
