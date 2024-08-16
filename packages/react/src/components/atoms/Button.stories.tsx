import type { Meta, StoryObj } from "@storybook/react";
import { Button } from "./Button";

const meta: Meta<typeof Button> = {
  title: "Design System/Atoms/Button",
  component: Button,
  argTypes: {
    variant: {
      options: ["default", "flat", "danger", "outline", "secondary", "ghost", "link"],
      control: { type: "select" },
      description: "The variant of the button.",
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
    variant: "default",
    size: "default",
    isDisabled: false,
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
    variant: "default",
    children: "Button",
  },
};

export const Flat: Store = {
  args: {
    variant: "flat",
    children: "Button",
  },
};
