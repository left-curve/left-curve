import { forwardRef, useImperativeHandle } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useConnectors } from "@left-curve/store";

import { Button, IconButton, IconClose, useApp } from "@left-curve/applets-kit";

import type { ModalRef } from "./RootModal";
import type { EIP1193Provider } from "@left-curve/store/types";
import { Image } from "~/components/foundation/Image";

type DestinationWalletProps = {
  onAddressSet: (address: string, walletName?: string, walletIcon?: string) => void;
};

const HIDDEN_CONNECTOR_TYPES = ["passkey", "session", "privy", "debug"];

export const DestinationWallet = forwardRef<ModalRef, DestinationWalletProps>(
  ({ onAddressSet }, ref) => {
    const { hideModal } = useApp();
    const connectors = useConnectors();

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => {},
    }));

    const filteredConnectors = connectors.filter((c) => !HIDDEN_CONNECTOR_TYPES.includes(c.type));

    const handleConnectorClick = async (connector: (typeof filteredConnectors)[number]) => {
      try {
        const provider = await (
          connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
        ).getProvider();
        const accounts = (await provider.request({ method: "eth_requestAccounts" })) as string[];
        const walletAddress = accounts[0];
        if (walletAddress) {
          onAddressSet(walletAddress, connector.name, connector.icon);
          hideModal();
        }
      } catch {
        // User rejected or error
      }
    };

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-4 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <p className="text-ink-primary-900 diatype-lg-medium w-full text-center">
          {m["bridge.destinationWallet"]()}
        </p>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={hideModal}
        >
          <IconClose className="w-5 h-5 text-ink-tertiary-500" />
        </IconButton>
        <div className="flex flex-col gap-3">
          {filteredConnectors.map((connector) => (
            <Button
              key={connector.uid}
              variant="secondary"
              fullWidth
              onClick={() => handleConnectorClick(connector)}
              className="flex items-center gap-3 justify-center"
            >
              {connector.icon && (
                <Image src={connector.icon} alt={connector.name} className="w-5 h-5" />
              )}
              <span>{connector.name}</span>
            </Button>
          ))}
        </div>
      </div>
    );
  },
);
