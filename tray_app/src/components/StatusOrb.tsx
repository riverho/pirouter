import React from "react";
import { Activity, Pause, AlertTriangle, Power } from "lucide-react";

type OrbState = "running" | "stopped" | "error" | "starting";

interface StatusOrbProps {
  state: OrbState;
  size?: number;
  className?: string;
}

const orbConfig: Record<OrbState, { bg: string; icon: React.ReactNode; glow: string }> = {
  running: {
    bg: "bg-pirouter-success",
    glow: "shadow-[0_0_16px_rgba(22,138,74,0.35)]",
    icon: <Activity className="w-5 h-5 text-white" strokeWidth={2} />,
  },
  stopped: {
    bg: "bg-pirouter-text-muted",
    glow: "",
    icon: <Pause className="w-5 h-5 text-white" strokeWidth={2} />,
  },
  error: {
    bg: "bg-pirouter-danger",
    glow: "shadow-[0_0_16px_rgba(220,38,38,0.35)]",
    icon: <AlertTriangle className="w-5 h-5 text-white" strokeWidth={2} />,
  },
  starting: {
    bg: "bg-pirouter-warning",
    glow: "shadow-[0_0_16px_rgba(183,121,31,0.35)]",
    icon: <Power className="w-5 h-5 text-white animate-pulse" strokeWidth={2} />,
  },
};

const StatusOrb: React.FC<StatusOrbProps> = ({ state, size = 32, className = "" }) => {
  const config = orbConfig[state];
  return (
    <div
      className={`rounded-full flex items-center justify-center ${config.bg} ${config.glow} ${className}`}
      style={{ width: size, height: size }}
      title={`Daemon ${state}`}
    >
      {config.icon}
    </div>
  );
};

export default StatusOrb;
