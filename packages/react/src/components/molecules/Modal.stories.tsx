import type { Meta, StoryObj } from "@storybook/react";

import type React from "react";
import { useRef } from "react";
import { Button, CloseIcon } from "../";
import { Modal, type ModalProps, type ModalRef } from "./Modal";

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

const Template: React.FC<ModalProps> = ({ onClose }) => {
  const modalRef = useRef<ModalRef>(null);

  return (
    <>
      <div>
        <Button onClick={() => [modalRef.current?.showModal(), console.log("test")]}>Open</Button>
        <Modal ref={modalRef} onClose={onClose}>
          <div className="relative flex flex-col items-center justify-center px-4 py-8 bg-slate-50 rounded-xl  min-h-[350px] min-w-[500px]">
            <CloseIcon
              className="absolute w-6 h-6 top-5 right-5 text-white hover:bg-primary-500 bg-slate-200 rounded-full cursor-pointer"
              onClick={() => modalRef.current?.closeModal()}
            />
            <p>Modal Example</p>
          </div>
        </Modal>
      </div>
    </>
  );
};
