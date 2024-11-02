import type { Meta, StoryObj } from "@storybook/react";
import { Button } from "./Button";

const meta: Meta<typeof Button> = {
  title: "Design System/Atoms/Button",
  component: Button,
  argTypes: {
    variant: {
      options: ["solid", "bordered", "light"],
      control: { type: "select" },
      description: "The variant of the button.",
    },
    color: {
      control: { type: "select" },
      description: "The color of the button.",
      options: ["none", "gray", "purple", "green", "rose", "sand"],
    },
    children: {
      control: { type: "text" },
      description: "This element could be a string or a React component.",
    },
    size: {
      options: ["sm", "lg"],
      control: { type: "select" },
      description: "The size of the button.",
    },
    isDisabled: {
      control: { type: "boolean" },
    },
  },
  args: {
    variant: "solid",
    size: "md",
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
    children: "Button",
  },
};

export const Bordered: Store = {
  args: {
    variant: "bordered",
    color: "purple",
    children: "Button",
  },
};

export const Light: Store = {
  args: {
    variant: "light",
    color: "rose",
    children: "Button",
  },
};
