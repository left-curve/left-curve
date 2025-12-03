import { forwardRef, useState } from "react";
import { Signup } from "../auth/Signup";
import { Signin } from "../auth/Signin";

import { IconButton, IconClose, useApp } from "@left-curve/applets-kit";

const views = {
  signin: Signin,
  signup: Signup,
};

export const Authenticate = forwardRef((_, __) => {
  const { hideModal } = useApp();
  const [view, setView] = useState("signin");

  const AuthView = views[view as keyof typeof views];

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
      <p className="text-ink-primary-900 diatype-lg-medium w-full text-center" />
      <div className=" flex flex-col gap-4 text-ink-primary-900">
        <AuthView goTo={(view) => setView(view)} />
      </div>
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
