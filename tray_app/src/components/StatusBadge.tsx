import React from "react";

type StatusVariant = "running" | "stopped" | "error" | "warning" | "success" | "muted";

interface StatusBadgeProps {
  variant: StatusVariant;
  children: React.ReactNode;
  className?: string;
}

const variantMap: Record<StatusVariant, { dot: string; bg: string; text: string }> = {
  running: { dot: "bg-pirouter-success", bg: "bg-pirouter-success-soft", text: "text-pirouter-success" },
  stopped: { dot: "bg-pirouter-text-muted", bg: "bg-pirouter-surface-muted", text: "text-pirouter-text-muted" },
  error: { dot: "bg-pirouter-danger", bg: "bg-pirouter-danger-soft", text: "text-pirouter-danger" },
  warning: { dot: "bg-pirouter-warning", bg: "bg-pirouter-warning-soft", text: "text-pirouter-warning" },
  success: { dot: "bg-pirouter-success", bg: "bg-pirouter-success-soft", text: "text-pirouter-success" },
  muted: { dot: "bg-pirouter-text-muted", bg: "bg-pirouter-surface-muted", text: "text-pirouter-text-muted" },
};

const StatusBadge: React.FC<StatusBadgeProps> = ({ variant, children, className = "" }) => {
  const v = variantMap[variant];
  return (
    <span
      className={`inline-flex items-center gap-1.5 h-[22px] px-2 rounded-badge text-metadata font-semibold ${v.bg} ${v.text} ${className}`}
    >
      <span className={`w-2 h-2 rounded-full ${v.dot}`} />
      {children}
    </span>
  );
};

export default StatusBadge;
