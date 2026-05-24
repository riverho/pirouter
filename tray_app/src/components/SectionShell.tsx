import React from "react";

interface SectionShellProps {
  title: string;
  action?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}

const SectionShell: React.FC<SectionShellProps> = ({ title, action, children, className = "" }) => {
  return (
    <div
      className={`bg-pirouter-surface border border-pirouter-border rounded-section shadow-section ${className}`}
    >
      {/* Section Header */}
      <div className="flex items-center justify-between h-7 px-section-pad pt-section-pad pb-label-gap">
        <h2 className="text-section-title text-pirouter-text tracking-normal">{title}</h2>
        {action && <div className="flex items-center">{action}</div>}
      </div>
      {/* Section Body */}
      <div className="px-section-pad pb-section-pad gap-3 flex flex-col">
        {children}
      </div>
    </div>
  );
};

export default SectionShell;
