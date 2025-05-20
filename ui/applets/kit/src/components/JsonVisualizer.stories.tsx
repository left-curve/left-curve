import type { Meta, StoryObj } from "@storybook/react";
import { JsonVisualizer } from "./JsonVisualizer";

const meta: Meta<typeof JsonVisualizer> = {
  title: "Design System/Foundation/JsonVisualizer",
  component: JsonVisualizer,
  argTypes: {
    json: {
      control: { type: "text" },
      description: "Json use in the JsonVisualizer",
    },
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof JsonVisualizer>;

export const Default: Store = {
  args: {
    json: `{
      "name": "John Doe",
      "age": 30,
      "location": {
        "city": "New York",
        "state": "NY"
      }
    }`,
  },
};
