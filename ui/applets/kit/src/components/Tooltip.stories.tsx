import type { Meta, StoryObj } from "@storybook/react";
import { Tooltip, type TooltipProps } from "./Tooltip";

const meta: Meta<typeof Tooltip> = {
  title: "Design System/Foundation/Tooltip",
  component: Tooltip,
  argTypes: {
    placement: {
      control: { type: "select" },
      options: ["top", "bottom", "left", "right", "auto"],
      description: "Placement of the tooltip relative to the trigger element.",
    },
    delay: {
      control: { type: "number" },
      description: "Delay before the tooltip appears in milliseconds.",
    },
    closeDelay: {
      control: { type: "number" },
      description: "Delay before the tooltip disappears in milliseconds.",
    },
    className: {
      control: { type: "text" },
      description: "Custom class name for the tooltip.",
    },
  },
  args: {
    content: "This is a tooltip example.",
    placement: "top",
    delay: 0,
    closeDelay: 50,
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Tooltip>;

export const Default: Store = {
  render: (args) => <Template {...args} />,
};

const Template: React.FC<TooltipProps> = (args) => {
  return (
    <div className="flex items-center justify-center relative py-10">
      <Tooltip {...args}>
        <p>Hover me!</p>
      </Tooltip>
    </div>
  );
};
