import type { Preview } from "@storybook/react-vite";

import "../src/index.css";
import { CompanionBridgeProvider } from "../src/app/companion-bridge-ui";
import { ToastProvider } from "../src/ui/toast/ToastProvider";

const ISOHUB_VIEWPORTS = {
  isohubNarrow: {
    name: "IsoHub Narrow (360×640)",
    styles: {
      width: "360px",
      height: "640px",
    },
    type: "mobile",
  },
  isohubMobile: {
    name: "IsoHub Mobile (390×844)",
    styles: {
      width: "390px",
      height: "844px",
    },
    type: "mobile",
  },
  isohubTablet: {
    name: "IsoHub Tablet (768×800)",
    styles: {
      width: "768px",
      height: "800px",
    },
    type: "tablet",
  },
  isohubCompactDesktop: {
    name: "IsoHub Compact Desktop (1024×700)",
    styles: {
      width: "1024px",
      height: "700px",
    },
    type: "desktop",
  },
  isohubLaptop: {
    name: "IsoHub Laptop (1280×800)",
    styles: {
      width: "1280px",
      height: "800px",
    },
    type: "desktop",
  },
  isohubDesktop: {
    name: "IsoHub Desktop (1440×900)",
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
      viewports: ISOHUB_VIEWPORTS,
      defaultViewport: "isohubDesktop",
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
