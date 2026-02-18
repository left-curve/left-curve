import { forwardRef, useImperativeHandle } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  Button,
  IconButton,
  IconClose,
  IconWarningTriangle,
  useApp,
} from "@left-curve/applets-kit";

import type { ModalRef } from "./RootModal";

export const AddressWarning = forwardRef<ModalRef>((_, ref) => {
  const { hideModal, navigate } = useApp();

  useImperativeHandle(ref, () => ({
    triggerOnClose: () => {},
  }));

  const handleGoToDeposit = () => {
    navigate("/bridge");
    hideModal();
  };

  return (
    <div className="flex flex-col bg-surface-primary-rice rounded-xl relative max-w-[400px]">
      <IconButton
        className="hidden lg:block absolute right-3 top-3"
        variant="link"
        onClick={hideModal}
      >
        <IconClose className="w-5 h-5 text-ink-tertiary-500" />
      </IconButton>
      <div className="p-6 flex flex-col gap-4">
        <div className="w-12 h-12 rounded-full bg-utility-warning-100 flex items-center justify-center">
          <IconWarningTriangle className="w-6 h-6 text-utility-warning-500" />
        </div>
        <div className="flex flex-col gap-2">
          <h3 className="diatype-lg-bold text-ink-primary-900">
            {m["accountCard.addressWarning.title"]()}
          </h3>
          <ul className="list-disc pl-5 text-ink-tertiary-500 diatype-m-regular flex flex-col gap-1">
            <li>
              <span className="diatype-m-bold">{m["accountCard.addressWarning.doNot"]()}</span>{" "}
              {m["accountCard.addressWarning.bullet1"]()}
            </li>
            <li>
              <span className="diatype-m-bold">{m["accountCard.addressWarning.doNot"]()}</span>{" "}
              {m["accountCard.addressWarning.bullet2"]()}
            </li>
          </ul>
          <p className="text-ink-tertiary-500 diatype-m-regular">
            {m["accountCard.addressWarning.descriptionPre"]()}{" "}
            <button type="button" className="text-ink-secondary-blue" onClick={handleGoToDeposit}>
              {m["accountCard.addressWarning.descriptionLink"]()}
            </button>{" "}
            {m["accountCard.addressWarning.descriptionPost"]()}
          </p>
        </div>
      </div>
      <div className="p-6 pt-0">
        <Button fullWidth variant="secondary" onClick={hideModal}>
          {m["accountCard.addressWarning.button"]()}
        </Button>
      </div>
    </div>
  );
});
