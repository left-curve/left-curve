"use client";

import { forwardRef, useRef } from "react";
import { WizardProvider } from "../../../providers";
import { mergeRefs } from "../../../utils";

import { Button, DangoButton, Modal, type ModalRef } from "../../";

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
          <DangoButton>Create Account</DangoButton>
        </DisplayIntro>
        <DisplayConnect />
        <DisplayConnection />
      </WizardProvider>
    </Modal>
  );
});

ConnectModal.displayName = "ConnectModal";
