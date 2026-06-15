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
  const devdOrigins = process.env.ISOHUB_DEVD_ORIGINS?.trim() ?? "";
  const devdOrigin = devdOrigins
    .split(",")
    .map((origin) => origin.trim())
    .find((origin) => origin.length > 0);
  const proxyLocalApi =
    command === "serve" &&
    mode !== "production" &&
    process.env.ISOHUB_DEV_PROXY !== "0";
  if (proxyLocalApi && !devdOrigin) {
    throw new Error(
      "ISOHUB_DEVD_ORIGINS is required for web dev proxy, for example ISOHUB_DEVD_ORIGINS=http://isohub-devd.local:51200,http://127.0.0.1:51200",
    );
  }

  return {
    base,
    define: {
      "import.meta.env.VITE_ISOHUB_DEVD_ORIGINS": JSON.stringify(devdOrigins),
    },
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
