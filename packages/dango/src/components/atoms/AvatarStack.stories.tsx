import type { Meta, StoryObj } from "@storybook/react";
import { AvatarStack } from "./AvatarStack";

const meta: Meta<typeof AvatarStack> = {
  title: "Design System/Atoms/AvatarStack",
  component: AvatarStack,
  argTypes: {
    images: {
      control: { type: "object" },
      description: "An array of image URLs.",
    },
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof AvatarStack>;

export const Default: Store = {
  args: {
    images: [
      "https://www.tapback.co/api/avatar/1.webp",
      "https://www.tapback.co/api/avatar/2.webp",
      "https://www.tapback.co/api/avatar/3.webp",
    ],
  },
};
