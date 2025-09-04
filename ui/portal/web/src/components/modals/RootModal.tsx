import { AnimatePresence } from "framer-motion";
import { motion } from "framer-motion";
import { Suspense, lazy, useMemo, useRef } from "react";

import { Button, Modals, useApp, useMediaQuery } from "@left-curve/applets-kit";
import type React from "react";
import { Sheet, type SheetRef } from "react-modal-sheet";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export type ModalRef = {
  triggerOnClose: () => void;
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
    component: lazy(() =>
      import("./SignWithDesktop").then(({ SignWithDesktop }) => ({
        default: SignWithDesktop,
      })),
    ),
    options: {
      header: m["common.signin"](),
    },
  },
  [Modals.ConfirmSwap]: {
    component: lazy(() =>
      import("./ConfirmSwap").then(({ ConfirmSwap }) => ({
        default: ConfirmSwap,
      })),
    ),
    options: {
      header: m["dex.convert.swap"](),
    },
  },
  [Modals.RenewSession]: {
    component: lazy(() =>
      import("./RenewSession").then(({ RenewSession }) => ({
        default: RenewSession,
      })),
    ),
    options: {
      disableClosing: true,
    },
  },
  [Modals.ProTradeCloseAll]: {
    component: lazy(() =>
      import("./ProTradeCloseAll").then(({ ProTradeCloseAll }) => ({ default: ProTradeCloseAll })),
    ),
  },
  [Modals.ProTradeCloseOrder]: {
    component: lazy(() =>
      import("./ProTradeCloseOrder").then(({ ProTradeCloseOrder }) => ({
        default: ProTradeCloseOrder,
      })),
    ),
  },
  [Modals.ProTradeLimitClose]: {
    component: lazy(() =>
      import("./ProTradeLimitClose").then(({ ProTradeLimitClose }) => ({
        default: ProTradeLimitClose,
      })),
    ),
  },
  [Modals.ProSwapMarketClose]: {
    component: lazy(() =>
      import("./ProSwapMarketClose").then(({ ProSwapMarketClose }) => ({
        default: ProSwapMarketClose,
      })),
    ),
  },
  [Modals.ProSwapEditTPSL]: {
    component: lazy(() =>
      import("./ProSwapEditTPSL").then(({ ProSwapEditTPSL }) => ({
        default: ProSwapEditTPSL,
      })),
    ),
  },
  [Modals.ProSwapEditedSL]: {
    component: lazy(() =>
      import("./ProSwapEditedSL").then(({ ProSwapEditedSL }) => ({
        default: ProSwapEditedSL,
      })),
    ),
  },
  [Modals.PoolAddLiquidity]: {
    component: lazy(() =>
      import("./PoolAddLiquidity").then(({ PoolAddLiquidity }) => ({
        default: PoolAddLiquidity,
      })),
    ),
  },
  [Modals.PoolWithdrawLiquidity]: {
    component: lazy(() =>
      import("./PoolWithdrawLiquidity").then(({ PoolWithdrawLiquidity }) => ({
        default: PoolWithdrawLiquidity,
      })),
    ),
  },
};

type ModalDefinition = {
  component: React.LazyExoticComponent<React.ForwardRefExoticComponent<any>>;
  options?: {
    header?: string;
    disableClosing?: boolean;
  };
};

export const RootModal: React.FC = () => {
  const { modal, hideModal } = useApp();
  const { isMd } = useMediaQuery();

  const sheetRef = useRef<SheetRef>();
  const overlayRef = useRef<HTMLDivElement>(null);
  const modalRef = useRef<{ triggerOnClose: () => void } | null>(null);

  const { modal: activeModal, props: modalProps } = modal;

  const { component: Modal, options = {} } =
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
        disableDrag={options.disableClosing}
        ref={sheetRef}
        isOpen={!!activeModal}
        onClose={closeModal}
        detent="content-height"
        rootId="root"
      >
        <Sheet.Container className="!bg-surface-primary-rice !rounded-t-2xl !shadow-none">
          <Sheet.Header>
            {options.header ? (
              <div className="flex items-center justify-between w-full">
                <Button variant="link" onClick={hideModal}>
                  {m["common.cancel"]()}
                </Button>
                <p className="mt-1 text-tertiary-500 font-semibold">{options.header}</p>
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
        <Sheet.Backdrop onTap={() => !options.disableClosing && closeModal()} />
      </Sheet>
    );
  }

  return (
    <AnimatePresence initial={false} mode="wait" onExitComplete={() => null}>
      <motion.div
        ref={overlayRef}
        onClick={(e) => {
          if (e.target === overlayRef.current && !options.disableClosing) closeModal();
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
