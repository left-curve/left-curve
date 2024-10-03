import type { Meta, StoryObj } from "@storybook/react";
import { AccountDescriptionCard } from "./AccountDescriptionCard";

const meta: Meta<typeof AccountDescriptionCard> = {
  title: "Design System/Atoms/AccountDescriptionCard",
  component: AccountDescriptionCard,
  argTypes: {
    title: {
      control: { type: "text" },
      description: "The title of the AccountDescriptionCard.",
    },
    img: {
      control: { type: "text" },
      description: "The image of the AccountDescriptionCard.",
    },
    description: {
      control: { type: "text" },
      description: "The description of the AccountDescription",
    },
    color: {
      control: { type: "select" },
      description: "The color of the AccountDescriptionCard.",
      options: ["default"],
    },
  },
  args: {
    title: "Spot account",
    description:
      "Can hold any asset and partake in any activity; cheapest gas cost; can only take over-collateralized loans.",
    img: "/images/avatars/spot.webp",
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof AccountDescriptionCard>;

export const Default: Store = {
  render: (args) => (
    <div className="w-full max-w-xl">
      <AccountDescriptionCard {...args} />
    </div>
  ),
};
