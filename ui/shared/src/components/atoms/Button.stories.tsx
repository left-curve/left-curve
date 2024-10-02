import type { Meta, StoryObj } from "@storybook/react";
import { Button } from "./Button";

const meta: Meta<typeof Button> = {
  title: "Design System/Atoms/Button",
  component: Button,
  argTypes: {
    variant: {
      options: ["solid", "outline", "light", "flat", "faded", "shadow", "dark", "ghost"],
      control: { type: "select" },
      description: "The variant of the button.",
    },
    color: {
      control: { type: "select" },
      description: "The color of the button.",
      options: ["default", "white", "purple", "green", "danger", "sand"],
    },
    children: {
      control: { type: "text" },
      description: "This element could be a string or a React component.",
    },
    size: {
      options: ["default", "sm", "lg", "icon", "none"],
      control: { type: "select" },
      description: "The size of the button.",
    },
    isDisabled: {
      control: { type: "boolean" },
    },
    asChild: {
      control: { type: "boolean" },
    },
  },
  args: {
    variant: "solid",
    color: "default",
    size: "default",
    asChild: false,
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Button>;

export const Default: Store = {
  args: {
    variant: "solid",
    color: "default",
    children: "Button",
  },
};

export const Flat: Store = {
  args: {
    variant: "flat",
    children: "Button",
  },
};
