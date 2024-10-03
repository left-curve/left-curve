import type { Meta, StoryObj } from "@storybook/react";
import { ProposalBar, type ProposalBarProps } from "./ProposalBar";

const meta: Meta<typeof ProposalBar> = {
  title: "Design System/Atoms/ProposalBar",
  component: ProposalBar,
  argTypes: {
    threshold: {
      control: { type: "number" },
      description: "The threshold of the account.",
    },
    totalWeight: {
      control: { type: "number" },
      description: "The totalWeight of the account.",
    },
    votes: {
      control: { type: "object" },
      description: "The votes in the proposal.",
    },
  },
  args: {
    votes: {
      positive: 10,
      negative: 5,
    },
    threshold: 7,
    totalWeight: 15,
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof ProposalBar>;

export const Default: Store = {
  render: (args) => <Template {...args} />,
};

const Template: React.FC<ProposalBarProps> = (args) => {
  return (
    <div className="min-w-72">
      <ProposalBar {...args} />
    </div>
  );
};
