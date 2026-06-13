import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// https://vite.dev/config/
export default defineConfig(({ command, mode }) => {
  const normalizeBase = (raw: string): string => {
    const trimmed = raw.trim();
    if (trimmed.length === 0) {
      return "/";
    }

    const withLeadingSlash = trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
    return withLeadingSlash.endsWith("/")
      ? withLeadingSlash
      : `${withLeadingSlash}/`;
  };

  const explicitBase = process.env.VITE_BASE;
  const repo = process.env.GITHUB_REPOSITORY?.split("/")[1];
  const base = explicitBase
    ? normalizeBase(explicitBase)
    : process.env.GITHUB_PAGES === "true" && repo
      ? `/${repo}/`
      : "/";
  const devdOrigin =
    process.env.ISOHUB_DEVD_ORIGIN?.trim() || "http://127.0.0.1:51200";
  const proxyLocalApi =
    command === "serve" &&
    mode !== "production" &&
    process.env.ISOHUB_DEV_PROXY !== "0";

  return {
    base,
    plugins: [react(), tailwindcss()],
    server: proxyLocalApi
      ? {
          proxy: {
            "/api/v1": {
              target: devdOrigin,
              changeOrigin: true,
            },
          },
        }
      : undefined,
  };
});
