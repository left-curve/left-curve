import { forwardRef } from "react";
import { AuthFlow } from "../auth/AuthFlow";

import { IconButton, IconClose, useApp } from "@left-curve/applets-kit";

type AuthenticateProps = {
  referrer?: number;
};

export const Authenticate = forwardRef<unknown, AuthenticateProps>(({ referrer }, _) => {
  const { hideModal } = useApp();

  return (
    <div className="flex flex-col justify-start items-center bg-surface-primary-rice text-ink-primary-900 md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative px-6 py-8 md:py-6 gap-5 md:w-[30rem] md:h-fit">
      <AuthFlow onFinish={hideModal} referrer={referrer} />
      <IconButton
        className="hidden md:block absolute right-4 top-4"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>
    </div>
  );
});
