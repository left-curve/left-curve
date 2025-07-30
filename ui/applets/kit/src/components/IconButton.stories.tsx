import type { Meta, StoryObj } from "@storybook/react";
import { IconButton, type IconButtonProps } from "./IconButton";
import { IconUser } from "./icons/IconUser";

const meta: Meta<typeof IconButton> = {
  title: "Design System/Foundation/IconButton",
  component: IconButton,
  argTypes: {
    variant: {
      options: ["primary", "secondary", "utility", "link"],
      control: { type: "select" },
      description: "The variant of the button.",
    },
    color: {
      control: { type: "select" },
      description: "The color of the button.",
      options: ["blue", "red", "green"],
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
    size: "lg",
    radius: "lg",
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof IconButton>;

const Template: React.FC<IconButtonProps> = (args) => {
  return (
    <IconButton {...args}>
      <IconUser className="w-6 h-6" />
    </IconButton>
  );
};

export const Primary: Store = {
  args: {
    variant: "primary",
  },
  render: (args) => <Template {...args} />,
};

export const Secondary: Store = {
  args: {
    variant: "secondary",
  },
  render: (args) => <Template {...args} />,
};

export const Utility: Store = {
  args: {
    variant: "utility",
  },
  render: (args) => <Template {...args} />,
};

export const Link: Store = {
  args: {
    variant: "link",
  },
  render: (args) => <Template {...args} />,
};
