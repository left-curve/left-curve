import { forwardRef, useImperativeHandle } from "react";
import { AuthOptions } from "../auth/AuthOptions";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { IconButton, IconClose, useApp } from "@left-curve/applets-kit";

type ConfirmSendProps = {
  onWalletSelect: (connectorId: string) => void;
  onReject: () => void;
};

export const WalletSelector = forwardRef<unknown, ConfirmSendProps>(
  ({ onWalletSelect, onReject }, ref) => {
    const { hideModal } = useApp();

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => onReject(),
    }));

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <p className="text-ink-primary-900 diatype-lg-medium w-full text-center">
          {m["modals.walletSelector.title"]()}
        </p>
        <div className=" flex flex-col gap-4">
          <AuthOptions
            action={(id) => {
              onWalletSelect(id);
              hideModal();
            }}
            isPending={false}
          />
        </div>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => [hideModal(), onReject()]}
        >
          <IconClose />
        </IconButton>
      </div>
    );
  },
);
