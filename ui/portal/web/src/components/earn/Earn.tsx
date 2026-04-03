import type { PropsWithChildren } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Button, createContext } from "@left-curve/applets-kit";

type EarnProps = {
  navigateToVault: () => void;
};

const [EarnProvider, useEarn] = createContext<EarnProps>({
  name: "EarnContext",
});

const EarnContainer: React.FC<PropsWithChildren<EarnProps>> = ({ children, navigateToVault }) => {
  return <EarnProvider value={{ navigateToVault }}>{children}</EarnProvider>;
};

const EarnHeader: React.FC = () => {
  return (
    <div className="flex flex-col items-center justify-center pb-6 text-center">
      <img
        src="/images/emojis/detailed/pig.svg"
        alt="pig-detailed"
        className="w-[148px] h-[148px] drag-none select-none"
      />
      <h1 className="exposure-h1-italic text-ink-primary-900">{m["earn.title"]()}</h1>
      <p className="text-ink-tertiary-500 diatype-lg-medium">{m["earn.description"]()}</p>
    </div>
  );
};

const EarnVaultCard: React.FC = () => {
  const { navigateToVault } = useEarn();

  return (
    <div className="flex justify-center p-4">
      <div className="flex flex-col gap-4 p-6 rounded-xl shadow-account-card bg-surface-tertiary-rice w-full max-w-[25rem] relative overflow-hidden">
        <div className="flex gap-2 items-center">
          <img
            src="/images/coins/usd.svg"
            alt="vault"
            className="w-10 h-10 rounded-full"
          />
          <div className="flex flex-col">
            <p className="text-ink-secondary-700 h4-bold">
              {m["vaultLiquidity.title"]()}
            </p>
          </div>
        </div>
        <div className="flex justify-between items-center">
          <div className="flex flex-col">
            <p className="text-ink-tertiary-500 diatype-xs-medium">{m["earn.apy"]()}</p>
            <p className="text-ink-secondary-700 diatype-sm-bold">-</p>
          </div>
          <div className="flex flex-col items-end">
            <p className="text-ink-tertiary-500 diatype-xs-medium">{m["earn.tvl"]()}</p>
            <p className="text-ink-secondary-700 diatype-sm-bold">-</p>
          </div>
        </div>
        <Button size="md" fullWidth onClick={navigateToVault}>
          {m["earn.select"]()}
        </Button>
        <img
          src="/images/characters/hippo.svg"
          alt="dango-hippo"
          className="max-w-[200px] absolute opacity-5 right-[-2rem] top-[-1rem] select-none drag-none"
        />
      </div>
    </div>
  );
};

export const Earn = Object.assign(EarnContainer, {
  Header: EarnHeader,
  VaultCard: EarnVaultCard,
});
