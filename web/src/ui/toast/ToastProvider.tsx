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

function toastClass(variant: ToastVariant): string {
  switch (variant) {
    case "success":
      return "iso-toast iso-toast--success";
    case "warning":
      return "iso-toast iso-toast--warning";
    case "error":
      return "iso-toast iso-toast--error";
    case "info":
      return "iso-toast iso-toast--info";
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
      <section className="iso-toast-stack" aria-label="Notifications">
        {toasts.map((t) => (
          <div
            key={t.id}
            className={toastClass(t.variant)}
            role={
              t.variant === "error" || t.variant === "warning"
                ? "alert"
                : "status"
            }
            aria-atomic="true"
          >
            <span>{t.message}</span>
          </div>
        ))}
      </section>
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
