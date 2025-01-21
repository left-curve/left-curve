import type { Meta, StoryObj } from "@storybook/react";
import { Input } from "./Input";

const meta: Meta<typeof Input> = {
  title: "Design System/Atoms/Input",
  component: Input,
  argTypes: {
    color: {
      control: { type: "select" },
      description: "The color of the input.",
      options: ["default"],
    },
    value: {
      control: { type: "text" },
      description: "This is the value in the input",
    },
    size: {
      options: ["md", "lg"],
      control: { type: "select" },
      description: "The size of the input.",
    },
    isDisabled: {
      control: { type: "boolean" },
    },
    fullWidth: {
      control: { type: "boolean" },
    },
    placeholder: {
      control: { type: "text" },
      description: "This is the placeholder in the input",
    },
  },
  args: {
    placeholder: "Input placeholder",
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Input>;

export const Default: Store = {
  args: {
    color: "default",
    size: "md",
    isDisabled: false,
    fullWidth: true,
  },
};
