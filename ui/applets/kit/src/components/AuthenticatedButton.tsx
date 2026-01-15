import { Modals, useApp } from "@left-curve/foundation";
import { useAccount } from "@left-curve/store";
import { cloneElement, type PropsWithChildren, type ReactElement } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

export const AuthenticatedButton: React.FC<PropsWithChildren> = ({ children }) => {
  const { showModal } = useApp();
  const { isConnected } = useAccount();
  if (isConnected) return children;

  const Button = cloneElement(
    children as ReactElement,
    {
      type: "button",
      onClick: () => showModal(Modals.Authenticate),
    },
    m["common.signin"](),
  );

  return Button;
};
