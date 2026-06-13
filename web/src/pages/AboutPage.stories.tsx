import type { Meta, StoryObj } from "@storybook/react";

import { AboutPage } from "./AboutPage";

const meta: Meta<typeof AboutPage> = {
  title: "Pages/AboutPage",
  component: AboutPage,
  tags: ["!test"],
  parameters: {
    layout: "padded",
  },
  decorators: [
    (Story) => (
      <div className="max-w-[980px]">
        <Story />
      </div>
    ),
  ],
};

export default meta;

type Story = StoryObj<typeof AboutPage>;

export const Default: Story = {};
