import type { Meta, StoryObj } from "@storybook/react";
import { Select, type SelectProps } from "./Select";

const meta: Meta<typeof Select> = {
  title: "Design System/Foundation/Select",
  component: Select,
  argTypes: {
    isDisabled: {
      control: { type: "boolean" },
      description: "Disabled state of the select",
    },
  },

  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Select>;

const Template: React.FC<SelectProps> = (args) => {
  return (
    <>
      <Select {...args}>
        <Select.Item value="1">1st Option</Select.Item>
        <Select.Item value="2">2nd Option</Select.Item>
      </Select>
    </>
  );
};

export const Default: Store = {
  render: (args) => <Template {...args} />,
};
