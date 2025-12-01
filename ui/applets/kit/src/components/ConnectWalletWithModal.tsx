import { useConnectors } from "@left-curve/store";
import { Button, type ButtonProps } from "./Button";
import { Modals, useApp } from "@left-curve/foundation";
import { withResolvers } from "@left-curve/dango/utils";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { EIP1193Provider } from "@left-curve/store/types";
import { useMutation } from "@tanstack/react-query";

interface ConnectWalletWithModalProps extends Omit<ButtonProps, "onClick" | "children"> {
  onWalletSelected: (walletId: string) => void;
}

export const ConnectWalletWithModal: React.FC<ConnectWalletWithModalProps> = ({
  onWalletSelected,
  ...buttonProps
}) => {
  const { showModal } = useApp();
  const connectors = useConnectors();

  const { isPending, mutateAsync } = useMutation({
    mutationFn: async () => {
      const { promise, resolve: onWalletSelect, reject: onReject } = withResolvers<string>();

      showModal(Modals.WalletSelector, {
        onWalletSelect,
        onReject,
      });

      const walletId = await promise;
      const connector = connectors.find((c) => c.id === walletId);
      if (!connector) onReject();

      try {
        const provider = await (
          connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
        ).getProvider();
        await provider.request({ method: "eth_requestAccounts" });
        onWalletSelected(walletId);
      } catch {
        onReject();
      }
    },
  });

  return (
    <Button {...buttonProps} onClick={() => mutateAsync()} isLoading={isPending}>
      {m["signin.connectWallet"]()}
    </Button>
  );
};
