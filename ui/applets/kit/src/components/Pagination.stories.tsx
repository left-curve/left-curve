import type { Meta, StoryObj } from "@storybook/react";
import { Pagination } from "./Pagination";

const meta: Meta<typeof Pagination> = {
  title: "Design System/Foundation/Pagination",
  component: Pagination,
  argTypes: {
    total: {
      control: { type: "number" },
      description: "The total number of pages.",
    },
    variant: {
      options: ["default", "text"],
      control: { type: "select" },
      description: "The variant of the pagination.",
    },
    siblings: {
      control: { type: "number" },
      description: "The number of pages to show before and after the current page.",
    },
    boundaries: {
      control: { type: "number" },
      description: "The number of pages to show at the beginning and end of the pagination.",
    },
    initialPage: {
      control: { type: "number" },
      description: "The initial page (uncontrolled).",
    },
    page: {
      control: { type: "number" },
      description: "The current page (controlled).",
    },
    onPageChange: {
      description: "Callback function to handle page changes.",
    },
    isDisabled: {
      control: { type: "boolean" },
      description: "Whether the pagination is disabled.",
    },
    id: {
      control: { type: "text" },
      description: "Optional ID for the pagination component if more than one is used.",
    },
  },
  args: {
    total: 10,
    isDisabled: false,
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Pagination>;

export const Primary: Store = {};
