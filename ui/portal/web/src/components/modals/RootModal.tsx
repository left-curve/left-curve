import { AnimatePresence } from "framer-motion";
import { motion } from "framer-motion";
import { Suspense, lazy, useMemo, useRef } from "react";
import { useApp } from "~/hooks/useApp";

import { Button, useMediaQuery } from "@left-curve/applets-kit";
import type React from "react";
import { Sheet, type SheetRef } from "react-modal-sheet";

import { m } from "~/paraglide/messages";

export const Modals = {
  AddKey: "add-key",
  RemoveKey: "remove-key",
  QRConnect: "qr-connect",
  ConfirmSend: "confirm-send",
  ConfirmAccount: "confirm-account",
  SignWithDesktop: "sign-with-desktop",
  SwapSummary: "swap-summary",
};

const modals: Record<(typeof Modals)[keyof typeof Modals], ModalDefinition> = {
  [Modals.AddKey]: {
    component: lazy(() => import("./AddKey").then(({ AddKeyModal }) => ({ default: AddKeyModal }))),
  },
  [Modals.RemoveKey]: {
    component: lazy(() => import("./RemoveKey").then(({ RemoveKey }) => ({ default: RemoveKey }))),
  },
  [Modals.QRConnect]: {
    component: lazy(() => import("./QRConnect").then(({ QRConnect }) => ({ default: QRConnect }))),
  },
  [Modals.ConfirmSend]: {
    component: lazy(() =>
      import("./ConfirmSend").then(({ ConfirmSend }) => ({ default: ConfirmSend })),
    ),
  },
  [Modals.ConfirmAccount]: {
    component: lazy(() =>
      import("./ConfirmAccount").then(({ ConfirmAccount }) => ({
        default: ConfirmAccount,
      })),
    ),
  },
  [Modals.SignWithDesktop]: {
    header: m["common.signin"](),
    component: lazy(() =>
      import("./SignWithDesktop").then(({ SignWithDesktop }) => ({
        default: SignWithDesktop,
      })),
    ),
  },
  [Modals.SwapSummary]: {
    header: "Swap",
    component: lazy(() =>
      import("./SwapSummary").then(({ SwapSummary }) => ({
        default: SwapSummary,
      })),
    ),
  },
};

type ModalDefinition = {
  header?: string;
  component: React.LazyExoticComponent<React.ForwardRefExoticComponent<any>>;
};

export const RootModal: React.FC = () => {
  const { modal, hideModal } = useApp();
  const { isMd } = useMediaQuery();

  const sheetRef = useRef<SheetRef>();
  const overlayRef = useRef<HTMLDivElement>(null);
  const modalRef = useRef<{ triggerOnClose: () => void } | null>(null);

  const { modal: activeModal, props: modalProps } = modal;

  const { component: Modal, header } =
    useMemo(() => modals[activeModal as keyof typeof modals], [activeModal, sheetRef]) || {};

  const closeModal = () => {
    hideModal();
    modalRef.current?.triggerOnClose();
  };

  if (!activeModal)
    return <AnimatePresence initial={false} mode="wait" onExitComplete={() => null} />;

  if (!isMd) {
    return (
      <Sheet
        ref={sheetRef}
        isOpen={!!activeModal}
        onClose={closeModal}
        detent="content-height"
        rootId="root"
      >
        <Sheet.Container className="!bg-white-100 !rounded-t-2xl !shadow-none">
          <Sheet.Header>
            {header ? (
              <div className="flex items-center justify-between w-full">
                <Button variant="link" onClick={hideModal}>
                  {m["common.cancel"]()}
                </Button>
                <p className="mt-1 text-gray-500 font-semibold">{header}</p>
                <div className="w-[66px]" />
              </div>
            ) : null}
          </Sheet.Header>
          <Sheet.Content>
            <Suspense>
              <Modal ref={modalRef} {...modalProps} />
            </Suspense>
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
        <Suspense>
          <Modal ref={modalRef} {...modalProps} />
        </Suspense>
      </motion.div>
    </AnimatePresence>
  );
};
