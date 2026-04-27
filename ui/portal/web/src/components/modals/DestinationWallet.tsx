import { forwardRef, useImperativeHandle, useState } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { isValidAddress } from "@left-curve/dango";
import { ethAddressMask } from "@left-curve/applets-kit";
import { useConnectors } from "@left-curve/store";

import {
  Button,
  IconButton,
  IconClose,
  IconWarningTriangle,
  Input,
  WarningContainer,
  useApp,
} from "@left-curve/applets-kit";

import type { ModalRef } from "./RootModal";
import type { EIP1193Provider } from "@left-curve/store/types";

type DestinationWalletProps = {
  network: string;
  onAddressSet: (address: string, walletName?: string, walletIcon?: string) => void;
};

type Step = "list" | "warning" | "input";

export const DestinationWallet = forwardRef<ModalRef, DestinationWalletProps>(
  ({ network, onAddressSet }, ref) => {
    const { hideModal } = useApp();
    const connectors = useConnectors();
    const [step, setStep] = useState<Step>("list");
    const [address, setAddress] = useState("");

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => {},
    }));

    const filteredConnectors = connectors.filter(
      (c) => c.type !== "passkey" && c.type !== "session" && c.type !== "privy",
    );

    const networkName = m["bridge.network"]({ network });

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

    const handleConfirm = () => {
      if (isValidAddress(address)) {
        onAddressSet(address);
        hideModal();
      }
    };

    if (step === "list") {
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
                  <img src={connector.icon} alt={connector.name} className="w-5 h-5" />
                )}
                <span>{connector.name}</span>
              </Button>
            ))}
            <Button
              variant="secondary"
              fullWidth
              onClick={() => setStep("warning")}
            >
              {m["bridge.enterAddressManually"]()}
            </Button>
          </div>
        </div>
      );
    }

    if (step === "warning") {
      return (
        <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-4 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
          <div className="w-10 h-10 rounded-full bg-surface-secondary-red flex items-center justify-center">
            <IconWarningTriangle className="w-5 h-5 text-utility-error-500" />
          </div>
          <IconButton
            className="hidden md:block absolute right-4 top-4"
            variant="link"
            onClick={hideModal}
          >
            <IconClose className="w-5 h-5 text-ink-tertiary-500" />
          </IconButton>
          <div className="flex flex-col gap-2">
            <p className="diatype-lg-bold text-ink-primary-900">
              {m["bridge.enterNetworkAddress"]({ network: networkName })}
            </p>
            <p className="text-ink-tertiary-500 diatype-m-regular">
              <strong>{m["bridge.riskOfFundLoss"]()}</strong>{" "}
              {m["bridge.riskOfFundLossDescription"]()}
            </p>
          </div>
          <Button fullWidth onClick={() => setStep("input")}>
            {m["bridge.iUnderstandTheRisk"]()}
          </Button>
        </div>
      );
    }

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-4 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <p className="text-ink-primary-900 diatype-lg-medium w-full text-center">
          {m["bridge.enterNetworkAddress"]({ network: networkName })}
        </p>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={hideModal}
        >
          <IconClose className="w-5 h-5 text-ink-tertiary-500" />
        </IconButton>
        <WarningContainer color="error" description={m["bridge.exchangeWarning"]()} />
        <Input
          label={m["bridge.withdrawAddress"]()}
          placeholder={m["bridge.placeholderWithdrawAddress"]({ network: networkName })}
          value={address}
          onChange={(e) => setAddress(ethAddressMask(e.target.value, address))}
        />
        <Button
          fullWidth
          onClick={handleConfirm}
          isDisabled={!isValidAddress(address)}
        >
          {m["bridge.confirm"]()}
        </Button>
      </div>
    );
  },
);
