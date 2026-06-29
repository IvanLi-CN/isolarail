import type { StorybookConfig } from "@storybook/react-vite";

process.env.ISOHUB_DEV_PROXY ??= "0";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.@(ts|tsx)"],
  addons: ["@storybook/addon-essentials"],
  framework: {
    name: "@storybook/react-vite",
    options: {},
  },
  core: {
    disableTelemetry: true,
  },
};

export default config;
