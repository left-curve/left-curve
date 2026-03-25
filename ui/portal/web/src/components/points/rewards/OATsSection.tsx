import { Button, useMediaQuery } from "@left-curve/applets-kit";
import { Modals, useApp } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { withResolvers } from "@left-curve/dango/utils";
import { useAccount, useConnectors, useRegisterOat } from "@left-curve/store";
import type { EIP1193Provider } from "@left-curve/store/types";
import type React from "react";
import { useState } from "react";
import { OATCard, type OATType } from "./OATCard";

type OATStatus = {
  type: OATType;
  isLocked: boolean;
  expiresAt?: number;
  pointsBoost: number;
};

type OATsSectionProps = {
  oatStatuses: OATStatus[];
};

export const OATsSection: React.FC<OATsSectionProps> = ({ oatStatuses }) => {
  const { isLg } = useMediaQuery();
  const { showModal, hideModal } = useApp();
  const connectors = useConnectors();
  const { userIndex, isConnected } = useAccount();
  const [isLinking, setIsLinking] = useState(false);
  const pointsUrl = window.dango.urls.pointsUrl;

  const { registerOat, isLoading: isRegistering } = useRegisterOat({
    pointsUrl,
    userIndex,
    onSuccess: () => {
      hideModal();
    },
    onError: (error) => {
      console.error("OAT registration failed:", error);
    },
  });

  const handleLinkWallet = async () => {
    if (!userIndex) return;

    setIsLinking(true);

    try {
      const { promise, resolve: onWalletSelect, reject: onReject } = withResolvers<string>();

      showModal(Modals.WalletSelector, {
        onWalletSelect,
        onReject,
      });

      const walletId = await promise;
      const connector = connectors.find((c) => c.id === walletId);
      if (!connector) {
        onReject();
        return;
      }

      const provider = await (
        connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
      ).getProvider();
      await provider.request({ method: "eth_requestAccounts" });

      await registerOat(walletId);
    } catch (error) {
      console.error("Wallet linking failed:", error);
    } finally {
      setIsLinking(false);
    }
  };

  const isButtonLoading = isLinking || isRegistering;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <p className="h4-bold text-ink-primary-900">{m["points.boosters.title"]()}</p>
        <p className="diatype-m-medium text-ink-tertiary-500">
          {m["points.boosters.description"]()}
        </p>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 lg:gap-8">
        {oatStatuses.map((oat) => (
          <OATCard
            key={oat.type}
            type={oat.type}
            isLocked={oat.isLocked}
            expiresAt={oat.expiresAt}
            pointsBoost={oat.pointsBoost}
          />
        ))}
      </div>

      <Button
        size={isLg ? "md" : "lg"}
        variant="primary"
        onClick={handleLinkWallet}
        isLoading={isButtonLoading}
        isDisabled={!isConnected}
        className="w-fit min-w-[8.3125rem]"
      >
        {m["points.boosters.linkEvmWallet"]()}
      </Button>
    </div>
  );
};
