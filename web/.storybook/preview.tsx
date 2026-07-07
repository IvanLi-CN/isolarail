import type { Preview } from "@storybook/react-vite";

import "../src/index.css";
import { CompanionBridgeProvider } from "../src/app/companion-bridge-ui";
import { ToastProvider } from "../src/ui/toast/ToastProvider";

const ISOLARAIL_VIEWPORTS = {
  isolarailNarrow: {
    name: "IsolaRail Narrow (360×640)",
    styles: {
      width: "360px",
      height: "640px",
    },
    type: "mobile",
  },
  isolarailMobile: {
    name: "IsolaRail Mobile (390×844)",
    styles: {
      width: "390px",
      height: "844px",
    },
    type: "mobile",
  },
  isolarailTablet: {
    name: "IsolaRail Tablet (768×800)",
    styles: {
      width: "768px",
      height: "800px",
    },
    type: "tablet",
  },
  isolarailCompactDesktop: {
    name: "IsolaRail Compact Desktop (1024×700)",
    styles: {
      width: "1024px",
      height: "700px",
    },
    type: "desktop",
  },
  isolarailLaptop: {
    name: "IsolaRail Laptop (1280×800)",
    styles: {
      width: "1280px",
      height: "800px",
    },
    type: "desktop",
  },
  isolarailDesktop: {
    name: "IsolaRail Desktop (1440×900)",
    styles: {
      width: "1440px",
      height: "900px",
    },
    type: "desktop",
  },
} as const;

const preview: Preview = {
  decorators: [
    (Story) => (
      <CompanionBridgeProvider>
        <ToastProvider>
          <Story />
        </ToastProvider>
      </CompanionBridgeProvider>
    ),
  ],
  parameters: {
    viewport: {
      viewports: ISOLARAIL_VIEWPORTS,
      defaultViewport: "isolarailDesktop",
    },
    actions: { argTypesRegex: "^on[A-Z].*" },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
  },
};

export default preview;
