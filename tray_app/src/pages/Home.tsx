import React, { useCallback, useEffect, useState } from "react";
import TopHeader from "@/components/TopHeader";
import SaveBar from "@/components/SaveBar";
import LedgerDrawer from "@/components/LedgerDrawer";
import DaemonSection from "@/sections/DaemonSection";
import RoutingProfileSection from "@/sections/RoutingProfileSection";
import ProvidersSection from "@/sections/ProvidersSection";
import ModelsSection from "@/sections/ModelsSection";
import RecentActivitySection from "@/sections/RecentActivitySection";
import RulesSection from "@/sections/RulesSection";
import {
  type AppState,
  loadAppState,
  quitApp,
  runDaemonAction,
  testProvider,
  validateConfig,
} from "@/api/pirouter";

type ValidationStatus = "clean" | "unsaved" | "validating" | "invalid" | "saved";

const Home: React.FC = () => {
  const [ledgerOpen, setLedgerOpen] = useState(false);
  const [appState, setAppState] = useState<AppState | null>(null);
  const [loading, setLoading] = useState(true);
  const [notice, setNotice] = useState<string | null>(null);
  const [validationStatus, setValidationStatus] = useState<ValidationStatus>("unsaved");
  // Tracks whether the user has manually dismissed the SaveBar for the
  // current daemon-stopped episode. Resets whenever daemon status changes.
  const [saveBarDismissed, setSaveBarDismissed] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setAppState(await loadAppState());
      setNotice(null);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleDaemonAction = async (action: "start" | "stop" | "restart") => {
    setNotice(null);
    try {
      const result = await runDaemonAction(action);
      setNotice(result);
      await refresh();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
      await refresh();
    }
  };

  // Validate config and surface result in the SaveBar + notice strip.
  const handleValidate = async () => {
    setValidationStatus("validating");
    try {
      const result = await validateConfig();
      setNotice(result);
      setValidationStatus("saved");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
      setValidationStatus("invalid");
    }
  };

  // Save = validate (no editable fields yet; wires the button to something useful).
  const handleSave = async () => {
    await handleValidate();
  };

  // Save & Restart = validate config first, then restart daemon if clean.
  const handleSaveAndRestart = async () => {
    setValidationStatus("validating");
    try {
      const result = await validateConfig();
      setNotice(result);
      setValidationStatus("saved");
      await handleDaemonAction("restart");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
      setValidationStatus("invalid");
    }
  };

  const handleTestProvider = async (id: string) => {
    try {
      setNotice(await testProvider(id));
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  };

  const handleQuitApp = async () => {
    try {
      await quitApp();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  };

  const daemon = appState?.daemon;
  const routing = appState?.routing;

  // Reset dismiss flag whenever the daemon status changes (e.g. user stopped
  // it after it was running — the bar should re-appear).
  const prevStatusRef = React.useRef(daemon?.status);
  if (prevStatusRef.current !== daemon?.status) {
    prevStatusRef.current = daemon?.status;
    if (saveBarDismissed) setSaveBarDismissed(false);
  }

  // Show the SaveBar whenever the daemon is not running — it doubles as a
  // "daemon is down, restart it" prompt with Save & Restart as the primary CTA.
  const daemonStopped = !loading && daemon?.status !== "running";
  const showSaveBar = daemonStopped && !saveBarDismissed;

  return (
    <div
      className="min-h-screen bg-pirouter-bg text-pirouter-text"
      style={{ paddingBottom: showSaveBar ? "80px" : "16px" }}
    >
      {/* Content container */}
      <div className="mx-auto p-page-gap" style={{ maxWidth: "1120px" }}>
        {/* Top header */}
        <TopHeader
          daemonStatus={daemon?.status ?? (loading ? "stopped" : "error")}
          currentProfile={routing?.profile ?? "unknown"}
          endpoint={daemon?.endpoint ?? "http://127.0.0.1:11435/v1"}
          onStop={() => void handleDaemonAction("stop")}
          onRestart={() => void handleDaemonAction("restart")}
          onOpenLogs={() => setNotice(daemon?.ledgerPath ?? "Ledger path unavailable")}
          onQuit={() => void handleQuitApp()}
        />

        {notice && (
          <div className="mt-3 min-h-9 rounded-section border border-pirouter-border bg-pirouter-surface px-3 py-2 text-metadata text-pirouter-text">
            {notice}
          </div>
        )}

        {/* Main content grid */}
        <div className="mt-page-gap grid grid-cols-12 gap-page-gap">
          {/* Left column: 7 columns */}
          <div className="col-span-7 flex flex-col gap-section-gap">
            <DaemonSection
              daemon={daemon}
              loading={loading}
              onStart={() => void handleDaemonAction("start")}
              onStop={() => void handleDaemonAction("stop")}
              onRestart={() => void handleDaemonAction("restart")}
              onOpenLogs={() => setNotice(daemon?.ledgerPath ?? "Ledger path unavailable")}
            />
            <ProvidersSection providers={appState?.providers ?? []} onTestProvider={handleTestProvider} />
            <ModelsSection models={appState?.models ?? []} />
          </div>

          {/* Right column: 5 columns */}
          <div className="col-span-5 flex flex-col gap-section-gap">
            <RoutingProfileSection routing={routing} />
            <RecentActivitySection
              rows={(appState?.ledger ?? []).slice(0, 4)}
              onOpenLedger={() => setLedgerOpen(true)}
            />
            <RulesSection rules={appState?.rules ?? []} />
          </div>
        </div>
      </div>

      {/* Save bar — visible when daemon is stopped or in error state.
          Save & Restart validates config then starts the daemon. */}
      <SaveBar
        visible={showSaveBar}
        validationStatus={validationStatus}
        restartRequired={daemon?.status === "stopped"}
        onDiscard={() => setSaveBarDismissed(true)}
        onValidate={() => void handleValidate()}
        onSave={() => void handleSave()}
        onSaveAndRestart={() => void handleSaveAndRestart()}
      />

      {/* Ledger Drawer */}
      <LedgerDrawer
        open={ledgerOpen}
        rows={appState?.ledger ?? []}
        endpoint={daemon?.endpoint ?? "http://127.0.0.1:11435/v1"}
        onRefresh={() => void refresh()}
        onClose={() => setLedgerOpen(false)}
      />
    </div>
  );
};

export default Home;
