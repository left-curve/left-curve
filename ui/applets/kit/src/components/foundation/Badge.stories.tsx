import type { Meta, StoryObj } from "@storybook/react";
import { Badge } from "./Badge";

const meta: Meta<typeof Badge> = {
  title: "Design System/Foundation/Badge",
  component: Badge,
  argTypes: {
    text: {
      control: { type: "text" },
      description: "Text use in the badge",
    },
    size: {
      options: ["s", "m"],
      control: { type: "select" },
      description: "The size of the badge.",
    },
    color: {
      options: ["red", "blue", "green"],
      control: { type: "select" },
      description: "The color of the badge.",
    },
  },
  args: {
    size: "s",
    text: "Badge",
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Badge>;

export const Red: Store = {
  args: {
    color: "red",
  },
};

export const Blue: Store = {
  args: {
    color: "blue",
  },
};

export const Green: Store = {
  args: {
    color: "green",
  },
};
