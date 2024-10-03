import type { Meta, StoryObj } from "@storybook/react";
import { TabItem } from "./TabItem";
import { Tabs, type TabsProps } from "./Tabs";

const meta: Meta<typeof Tabs> = {
  title: "Design System/Atoms/Tabs",
  component: Tabs,
  argTypes: {},
  args: {},
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
      <Tabs {...args}>
        <TabItem title="1st Tab">I'm rendering 1st Tab</TabItem>
        <TabItem title="2nd Tab">I'm rendering 2st Tab</TabItem>
      </Tabs>
    </>
  );
};
