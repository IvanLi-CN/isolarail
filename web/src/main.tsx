import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App.tsx";
import { initThemeFromStorage } from "./app/theme";
import "./index.css";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Missing root element");
}

initThemeFromStorage();

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
