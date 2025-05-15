import type { Meta, StoryObj } from "@storybook/react";
import { Pagination } from "./Pagination";

const meta: Meta<typeof Pagination> = {
  title: "Design System/Foundation/Pagination",
  component: Pagination,
  argTypes: {
    totalPages: {
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
    currentPage: {
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
    labelPage: {
      control: { type: "text" },
      description: "Label for word 'Page' when variant is 'text'.",
    },
    labelOf: {
      control: { type: "text" },
      description: "Label for word 'of' when variant is 'text'.",
    },
  },
  args: {
    totalPages: 15,
    isDisabled: false,
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Pagination>;

export const Default: Store = {
  args: {
    totalPages: 15,
    siblings: 1,
    boundaries: 1,
    initialPage: 1,
  },
};
export const Text: Store = {
  args: {
    variant: "text",
    totalPages: 10,
    siblings: 1,
    boundaries: 1,
    initialPage: 1,
  },
};
