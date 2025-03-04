import type { Meta, StoryObj } from "@storybook/react";
import { Spinner } from "./Spinner";

const meta: Meta<typeof Spinner> = {
  title: "Design System/Foundation/Spinner",
  component: Spinner,
  argTypes: {
    color: {
      control: { type: "select" },
      description: "The color of the Spinner.",
      options: ["current", "white", "pink", "green"],
    },
    size: {
      options: ["sm", "md", "lg"],
      control: { type: "select" },
      description: "The size of the Spinner.",
    },
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Spinner>;

export const Default: Store = {
  args: {
    color: "pink",
    size: "md",
  },
};
