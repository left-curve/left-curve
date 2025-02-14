import type { Meta, StoryObj } from "@storybook/react";
import { TruncateText } from "./TruncateText";

const meta: Meta<typeof TruncateText> = {
  title: "Design System/Foundation/TruncateText",
  component: TruncateText,
  argTypes: {
    start: {
      description: "(Optional) The number of characters to show at the start of the text.",
      control: { type: "number" },
      table: {
        defaultValue: { summary: "8" },
      },
    },
    end: {
      description: "(Optional) The number of characters to show at the end of the text.",
      control: { type: "number" },
      table: {
        defaultValue: { summary: "8" },
      },
    },
    text: {
      description: "The text to truncate.",
      control: { type: "text" },
    },
  },

  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof TruncateText>;

export const Default: Store = {
  args: {
    text: "0x1234567890abcdef567890abcdef1234567890abcdef567890abcdef",
    start: 8,
    end: 8,
  },
};
