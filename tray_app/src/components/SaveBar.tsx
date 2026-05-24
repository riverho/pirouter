import React from "react";
import { AlertCircle, Check } from "lucide-react";

interface SaveBarProps {
  visible: boolean;
  validationStatus?: "clean" | "unsaved" | "validating" | "invalid" | "saved";
  restartRequired?: boolean;
  onValidate?: () => void;
  onSave?: () => void;
  onSaveAndRestart?: () => void;
  onDiscard?: () => void;
}

const SaveBar: React.FC<SaveBarProps> = ({
  visible,
  validationStatus = "unsaved",
  restartRequired = false,
  onValidate,
  onSave,
  onSaveAndRestart,
  onDiscard,
}) => {
  if (!visible) return null;

  return (
    <div className="fixed bottom-4 left-4 right-4 h-16 rounded-section bg-pirouter-text shadow-save-bar z-50 flex items-center px-6 gap-4">
      {/* Left: Status */}
      <div className="flex-1 flex flex-col justify-center gap-0.5 min-w-0">
        <span className="text-body text-white font-semibold flex items-center gap-2">
          <AlertCircle className="w-4 h-4 text-pirouter-warning shrink-0" />
          Unsaved changes
        </span>
        <span className="text-metadata text-white/60">
          {validationStatus === "invalid"
            ? "Config validation failed"
            : restartRequired
            ? "Restart required for changes to take effect"
            : "Config validates"}
        </span>
      </div>

      {/* Right: Actions */}
      <div className="flex items-center gap-2 shrink-0">
        <button
          onClick={onDiscard}
          className="h-9 px-4 rounded-button text-button bg-transparent text-white/70 hover:text-white hover:bg-white/10 border border-white/20 transition-colors duration-120"
        >
          Discard
        </button>
        <button
          onClick={onValidate}
          className="h-9 px-4 rounded-button text-button bg-transparent text-white/70 hover:text-white hover:bg-white/10 border border-white/20 transition-colors duration-120"
        >
          Validate
        </button>
        <button
          onClick={onSave}
          className="h-9 px-4 rounded-button text-button bg-white/10 text-white hover:bg-white/20 border border-white/20 transition-colors duration-120"
        >
          Save
        </button>
        <button
          onClick={onSaveAndRestart}
          className="h-9 px-4 rounded-button text-button bg-pirouter-accent text-white hover:bg-[#0d8b7d] transition-colors duration-120 flex items-center gap-2"
        >
          <Check className="w-4 h-4" />
          Save & Restart
        </button>
      </div>
    </div>
  );
};

export default SaveBar;
