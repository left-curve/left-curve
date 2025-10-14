import { forwardRef } from "react";
import { IconButton, IconClose, useApp } from "@left-curve/applets-kit";

export const SignupReminder = forwardRef<undefined>(() => {
  const { hideModal } = useApp();

  return (
    <div className="w-full flex flex-col items-center justify-start bg-surface-primary-rice rounded-xl border border-outline-secondary-gray max-w-2xl">
      <div className="flex flex-col relative gap-4 p-4">
        <IconButton
          className="hidden md:block absolute right-2 top-2"
          variant="link"
          onClick={() => hideModal()}
        >
          <IconClose />
        </IconButton>
        <div className="w-12 h-12 rounded-full flex items-center justify-center">
          <img
            src="/favicon.svg"
            alt="dango logo"
            className={
              "h-11 order-1 cursor-pointer flex rounded-full shadow-account-card select-none"
            }
          />
        </div>
        <p className="h4-bold">Reminder</p>
        <div className="flex flex-col diatype-m-medium text-ink-tertiary-500 gap-4">
          <p>Hi there,</p>
          <p>
            This testnet comes with a{" "}
            <a href="https://app.galxe.com/quest/dango/GCNAXt8Tqv" target="_blank" rel="noreferrer">
              quest
            </a>{" "}
            hosted on the Galxe platform.{" "}
            <span className="font-bold">
              If Make sure to sign up with the Ethereum wallet with which you intend to claim the
              reward.
            </span>{" "}
            Users signed up using email or social media accounts won't be able to claim the rewards.
          </p>

          <p>Data from the previous testnets are not carried over. Please sign up again.</p>

          <p>Have fun and g🍡</p>
        </div>
      </div>
    </div>
  );
});
