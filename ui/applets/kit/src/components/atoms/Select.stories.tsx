import type { Meta, StoryObj } from "@storybook/react";
import { Item, Select, type SelectProps } from "./Select";

const meta: Meta<typeof Select> = {
  title: "Design System/Atoms/Select",
  component: Select,
  argTypes: {
    color: {
      control: { type: "select" },
      description: "The color of the Select.",
      options: ["default"],
    },
    size: {
      options: ["md", "lg"],
      control: { type: "select" },
      description: "The size of the Select.",
    },
    isDisabled: {
      control: { type: "boolean" },
    },
    placeholder: {
      control: { type: "text" },
      description: "This is the placeholder in the Select",
    },
  },
  args: {
    placeholder: "Select placeholder",
    label: "demo-select",
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Select>;

export const Default: Store = {
  render: (args) => <Template {...args} />,
};

const Template: React.FC<SelectProps<object>> = (args) => {
  return (
    <>
      <Select {...args}>
        <Item>1st Option</Item>
        <Item>2nd Option</Item>
      </Select>
    </>
  );
};
