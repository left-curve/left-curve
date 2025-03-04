import type { Meta, StoryObj } from "@storybook/react";
import { Input } from "./Input";
import { IconBell } from "./icons/IconBell";

const meta: Meta<typeof Input> = {
  title: "Design System/Foundation/Input",
  component: Input,
  argTypes: {
    value: {
      control: { type: "text" },
      description: "This is the value in the input",
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
    placeholder: "Placeholder",
    hintMessage: "Hint message",
    isDisabled: false,
    fullWidth: true,
  },
};

export const ErrorState: Store = {
  args: {
    placeholder: "Placeholder",
    errorMessage: "This is an error",
    isDisabled: false,
    fullWidth: true,
  },
};

export const StartContent: Store = {
  args: {
    placeholder: "Placeholder",
    isDisabled: false,
    fullWidth: true,
    startContent: <IconBell className="w-5 h-5" />,
  },
};

export const Disabled: Store = {
  args: {
    placeholder: "Placeholder",
    isDisabled: true,
    fullWidth: true,
  },
};
