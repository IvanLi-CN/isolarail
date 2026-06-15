import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
} from "react";

export type ToastVariant = "info" | "success" | "warning" | "error";

export type ToastInput = {
  message: string;
  variant?: ToastVariant;
  durationMs?: number;
};

type Toast = {
  id: string;
  message: string;
  variant: ToastVariant;
};

type ToastContextValue = {
  pushToast: (toast: ToastInput) => void;
};

const ToastContext = createContext<ToastContextValue | null>(null);

function alertClass(variant: ToastVariant): string {
  switch (variant) {
    case "success":
      return "alert-success";
    case "warning":
      return "alert-warning";
    case "error":
      return "alert-error";
    case "info":
      return "alert-info";
  }
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const pushToast = useCallback((input: ToastInput) => {
    const id = crypto.randomUUID();
    const variant = input.variant ?? "info";
    const durationMs = input.durationMs ?? 2500;

    setToasts((prev) => [...prev, { id, message: input.message, variant }]);
    window.setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, durationMs);
  }, []);

  const value = useMemo(() => ({ pushToast }), [pushToast]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="toast toast-end z-50">
        {toasts.map((t) => (
          <div key={t.id} className={`alert ${alertClass(t.variant)}`}>
            <span>{t.message}</span>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) {
    throw new Error("useToast must be used within <ToastProvider>");
  }
  return ctx;
}
