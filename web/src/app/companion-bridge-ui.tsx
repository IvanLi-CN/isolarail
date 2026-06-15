import { createContext, useContext, useEffect, useState } from "react";
import {
  type CompanionBridge,
  tryBootstrapCompanionBridge,
} from "../domain/companionBridge";

type CompanionBridgeStatus = "loading" | "ready";

type CompanionBridgeContextValue = {
  agent: CompanionBridge | null;
  status: CompanionBridgeStatus;
};

const CompanionBridgeContext =
  createContext<CompanionBridgeContextValue | null>(null);

export function CompanionBridgeProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const [agent, setAgent] = useState<CompanionBridge | null>(null);
  const [status, setStatus] = useState<CompanionBridgeStatus>("loading");

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const next = await tryBootstrapCompanionBridge();
      if (cancelled) {
        return;
      }
      setAgent(next);
      setStatus("ready");
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <CompanionBridgeContext.Provider value={{ agent, status }}>
      {children}
    </CompanionBridgeContext.Provider>
  );
}

export function useCompanionBridge(): CompanionBridgeContextValue {
  const ctx = useContext(CompanionBridgeContext);
  if (!ctx) {
    throw new Error(
      "useCompanionBridge must be used within <CompanionBridgeProvider>",
    );
  }
  return ctx;
}
