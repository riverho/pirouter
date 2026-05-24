import React, { useState } from "react";
import { Check, X, Loader2 } from "lucide-react";
import SectionShell from "@/components/SectionShell";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { ProviderInfo } from "@/api/pirouter";

interface ProvidersSectionProps {
  providers: ProviderInfo[];
  onTestProvider: (id: string) => Promise<void>;
}

const statusDotClass = (status: ProviderInfo["status"]) => {
  switch (status) {
    case "healthy": return "bg-pirouter-success";
    case "configured": return "bg-pirouter-accent";
    case "error": return "bg-pirouter-danger";
    case "testing": return "bg-pirouter-warning animate-pulse";
    default: return "bg-pirouter-border";
  }
};

const ProvidersSection: React.FC<ProvidersSectionProps> = ({ providers, onTestProvider }) => {
  const [testingId, setTestingId] = useState<string | null>(null);

  const handleTest = async (id: string) => {
    setTestingId(id);
    try {
      await onTestProvider(id);
    } finally {
      setTestingId(null);
    }
  };

  return (
    <SectionShell title="Providers">
      <div className="flex flex-col gap-dense-row">
        {/* Header row */}
        <div className="grid grid-cols-[48px_88px_1fr_100px_64px_56px] gap-2 items-center h-7">
          <span className="text-table-header text-pirouter-text-muted uppercase tracking-normal">On</span>
          <span className="text-table-header text-pirouter-text-muted uppercase tracking-normal">Provider</span>
          <span className="text-table-header text-pirouter-text-muted uppercase tracking-normal">Base URL</span>
          <span className="text-table-header text-pirouter-text-muted uppercase tracking-normal">Key Env</span>
          <span className="text-table-header text-pirouter-text-muted uppercase tracking-normal text-right">Timeout</span>
          <span className="text-table-header text-pirouter-text-muted uppercase tracking-normal text-right">Test</span>
        </div>

        {/* Provider rows */}
        {providers.map((provider) => (
          <div
            key={provider.id}
            className="grid grid-cols-[48px_88px_1fr_100px_64px_56px] gap-2 items-center h-14 bg-pirouter-surface-muted/50 rounded-input px-2"
          >
            {/* Enabled switch */}
            <Switch
              checked={provider.enabled}
              disabled
              className="data-[state=checked]:bg-pirouter-accent scale-90 origin-left"
            />
            {/* Provider name */}
            <div className="flex items-center gap-2 min-w-0">
              <span className={`w-2 h-2 rounded-full shrink-0 ${statusDotClass(provider.status)}`} />
              <span className="text-body text-pirouter-text truncate">{provider.name}</span>
            </div>
            {/* Base URL */}
            <input
              type="text"
              value={provider.baseUrl}
              readOnly
              className="h-8 px-2 rounded-input border border-pirouter-border bg-pirouter-surface text-metadata text-pirouter-text truncate focus:outline-none"
            />
            {/* Key Env */}
            <input
              type="text"
              value={provider.keyEnv}
              readOnly
              className="h-8 px-2 rounded-input border border-pirouter-border bg-pirouter-surface text-metadata text-pirouter-text truncate font-mono focus:outline-none"
            />
            {/* Timeout */}
            <span className="text-body text-pirouter-text tabular-nums text-right">{provider.timeout}s</span>
            {/* Test button */}
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => handleTest(provider.id)}
                    disabled={testingId === provider.id || !provider.enabled}
                    className="h-8 w-14 rounded-button text-button flex items-center justify-center border border-pirouter-border bg-pirouter-surface text-pirouter-text hover:bg-pirouter-surface-muted disabled:opacity-40 disabled:cursor-not-allowed transition-colors duration-120 ml-auto"
                  >
                    {testingId === provider.id ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : provider.status === "healthy" || provider.status === "configured" ? (
                      <Check className="w-4 h-4 text-pirouter-success" />
                    ) : provider.status === "error" ? (
                      <X className="w-4 h-4 text-pirouter-danger" />
                    ) : (
                      "Test"
                    )}
                  </button>
                </TooltipTrigger>
                <TooltipContent>Test connection</TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>
        ))}
      </div>
    </SectionShell>
  );
};

export default ProvidersSection;
