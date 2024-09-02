import type { Meta, StoryObj } from "@storybook/react";

import React, { useEffect } from "react";
import { Button } from "../atoms/Button";
import { Modal, type ModalProps, ModalRoot } from "./Modal";

const meta: Meta<typeof Modal> = {
  title: "Design System/Molecules/Modal",
  component: Modal,
  argTypes: {
    children: {
      control: { type: "object" },
      description: "This element is React component.",
    },
    onClose: {
      control: { type: "object" },
      description: "This function is called when the modal is closed.",
    },
    showModal: {
      control: { type: "boolean" },
      description: "This boolean is used to show or hide the modal.",
    },
  },
  args: {
    onClose: () => console.log("Modal closed"),
  },
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
};

export default meta;

type Store = StoryObj<typeof Modal>;

export const Default: Store = {
  render: (args) => <Template {...args} />,
};

const Template: React.FC<ModalProps> = ({ showModal: _showModal_, onClose }) => {
  const [showModal, setShowModal] = React.useState(false);

  useEffect(() => {
    setShowModal(_showModal_);
  }, [_showModal_]);

  return (
    <>
      <div>
        <Button onClick={() => setShowModal(true)}>Open</Button>
        <Modal showModal={showModal} onClose={() => [onClose?.(), setShowModal(false)]}>
          <p className="flex items-center justify-center px-4 py-8 bg-neutral-100 rounded-xl text-neutral-900 min-h-[350px] min-w-[500px]">
            This is a modal!
          </p>
        </Modal>
      </div>
      <ModalRoot />
    </>
  );
};
