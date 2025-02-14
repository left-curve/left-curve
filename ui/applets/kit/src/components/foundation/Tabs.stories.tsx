import type { Meta, StoryObj } from "@storybook/react";
import { Tabs, type TabsProps } from "./Tabs";

const meta: Meta<typeof Tabs> = {
  title: "Design System/Foundation/Tabs",
  component: Tabs,
  argTypes: {
    keys: {
      control: { type: "object" },
      description: "The keys of the Tabs.",
    },
    defaultKey: {
      control: { type: "text" },
      description: "The default key of the Tabs.",
    },
    onTabChange: {
      action: "onTabChange",
      description: "This function is called when a tab is changed.",
    },
  },
  args: {
    keys: ["Token", "Pools", "Earn"],
    defaultKey: "Pools",
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Tabs>;

export const Default: Store = {
  render: (args) => <Template {...args} />,
};

const Template: React.FC<TabsProps> = (args) => {
  return (
    <>
      <Tabs {...args} />
    </>
  );
};
