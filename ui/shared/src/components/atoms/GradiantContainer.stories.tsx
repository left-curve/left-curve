import type { Meta, StoryObj } from "@storybook/react";
import { GradientContainer } from "./GradientContainer";

const meta: Meta<typeof GradientContainer> = {
  title: "Design System/Atoms/GradientContainer",
  component: GradientContainer,
  argTypes: {
    children: {
      table: {
        type: { summary: "ReactNode" },
      },
      control: { type: "select" },
      description: "The react element inside the gradient container",
      options: ["empty", "content"],
      mapping: {
        empty: null,
        content: <div>This is an element in the gradient container</div>,
      },
    },
  },
  args: {
    children: "content",
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof GradientContainer>;

export const Default: Store = {};
