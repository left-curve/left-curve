"use client";

import { forwardRef, useRef } from "react";
import { WizardProvider } from "~/providers";
import { mergeRefs } from "~/utils";

import { Button, Modal, type ModalRef } from "~/components";

import { DisplayConnect } from "./DisplayConnect";
import { DisplayIntro } from "./DisplayIntro";
import { WrapperConnect } from "./WrapperConnect";

import { DisplayConnection } from "./DisplayConnection";

interface Props {
  challenge?: string;
}

export const ConnectModal = forwardRef<ModalRef, Props>(({ challenge }, ref) => {
  const modalRef = useRef<ModalRef>(null);

  const closeModal = () => {
    modalRef.current?.closeModal();
  };

  return (
    <Modal ref={mergeRefs(ref, modalRef)}>
      <WizardProvider onFinish={closeModal} onReset={closeModal} wrapper={<WrapperConnect />}>
        <DisplayIntro>
          <Button>Create Account</Button>
        </DisplayIntro>
        <DisplayConnect />
        <DisplayConnection />
      </WizardProvider>
    </Modal>
  );
});

ConnectModal.displayName = "ConnectModal";
