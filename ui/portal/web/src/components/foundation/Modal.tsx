import { AnimatePresence } from "framer-motion";
import { motion } from "framer-motion";
import { useMemo, useRef } from "react";
import { useApp } from "~/hooks/useApp";

import { useMediaQuery } from "@left-curve/applets-kit";
import type React from "react";
import { Sheet, type SheetRef } from "react-modal-sheet";

import { AddKeyModal } from "../modals/AddKey";
import { ConfirmAccount } from "../modals/ConfirmAccount";
import { ConfirmSend } from "../modals/ConfirmSend";
import { QRConnect } from "../modals/QRConnect";
import { RemoveKey } from "../modals/RemoveKey";

export const Modals = {
  AddKey: "add-key",
  RemoveKey: "remove-key",
  QRConnect: "qr-connect",
  ConfirmSend: "confirm-send",
  ConfirmAccount: "confirm-account",
};

const modals = {
  [Modals.AddKey]: {
    component: AddKeyModal,
    initialSnap: 0.7,
  },
  [Modals.RemoveKey]: {
    component: RemoveKey,
    initialSnap: 0.4,
  },
  [Modals.QRConnect]: {
    component: QRConnect,
    initialSnap: 0.4,
  },
  [Modals.ConfirmSend]: {
    component: ConfirmSend,
    initialSnap: 0.6,
  },
  [Modals.ConfirmAccount]: {
    component: ConfirmAccount,
    initialSnap: 0.5,
  },
};

export const Modal: React.FC = () => {
  const { activeModal, isModalVisible, hideModal, modalProps } = useApp();
  const { isMd } = useMediaQuery();

  const sheetRef = useRef<SheetRef>();
  const overlayRef = useRef<HTMLDivElement>(null);
  const modalRef = useRef<{ triggerOnClose: () => void }>();

  const { component: ModalContainer, initialSnap } =
    useMemo(() => modals[activeModal as keyof typeof modals], [activeModal, sheetRef]) || {};

  const closeModal = () => {
    hideModal();
    modalRef.current?.triggerOnClose();
  };

  if (!isModalVisible || !activeModal)
    return <AnimatePresence initial={false} mode="wait" onExitComplete={() => null} />;

  if (!isMd) {
    return (
      <Sheet
        ref={sheetRef}
        isOpen={isModalVisible}
        onClose={closeModal}
        initialSnap={0}
        snapPoints={[initialSnap]}
      >
        <Sheet.Container className="!bg-white-100 !rounded-t-2xl !shadow-none">
          <Sheet.Header />
          <Sheet.Content>
            <ModalContainer ref={modalRef} {...modalProps} />
          </Sheet.Content>
        </Sheet.Container>
        <Sheet.Backdrop onTap={closeModal} />
      </Sheet>
    );
  }

  return (
    <AnimatePresence initial={false} mode="wait" onExitComplete={() => null}>
      <motion.div
        ref={overlayRef}
        onClick={(e) => {
          if (e.target === overlayRef.current) closeModal();
        }}
        className="backdrop-blur-[10px] bg-gray-900/10 w-screen h-screen fixed top-0 z-[60] flex items-center justify-center p-4"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
      >
        <ModalContainer ref={modalRef} {...modalProps} />
      </motion.div>
    </AnimatePresence>
  );
};
