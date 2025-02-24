import type { Meta, StoryObj } from "@storybook/react";
import { Button } from "./Button";

const meta: Meta<typeof Button> = {
  title: "Design System/Foundation/Button",
  component: Button,
  argTypes: {
    variant: {
      options: ["primary", "secondary", "utility", "link"],
      control: { type: "select" },
      description: "The variant of the button.",
    },
    children: {
      control: { type: "text" },
      description: "This element could be a string or a React component.",
    },
    radius: {
      options: ["none", "sm", "md", "lg", "xl", "full"],
      control: { type: "select" },
      description: "The radius of the button.",
    },
    size: {
      options: ["xs", "sm", "md", "lg", "xl"],
      control: { type: "select" },
      description: "The size of the button.",
    },
    fullWidth: {
      control: { type: "boolean" },
    },
    isDisabled: {
      control: { type: "boolean" },
    },
  },
  args: {
    fullWidth: false,
    isDisabled: false,
    variant: "primary",
    children: "Button",
    size: "lg",
    radius: "lg",
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Button>;

export const Primary: Store = {};

export const Secondary: Store = {
  args: {
    variant: "secondary",
  },
};

export const Utility: Store = {
  args: {
    variant: "utility",
  },
};

export const Link: Store = {
  args: {
    variant: "link",
  },
};
