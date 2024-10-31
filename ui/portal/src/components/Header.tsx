import {
  CommandBar,
  ConnectButton,
  MenuAccounts,
  MenuConnections,
  MenuNotifications,
} from "@dango/shared";

import { AccountType } from "@leftcurve/types";

import { useAccount } from "@leftcurve/react";
import { Link, useNavigate } from "react-router-dom";
import { applets, popularApplets } from "../../applets";

export const Header: React.FC = () => {
  const navigate = useNavigate();
  const { account } = useAccount();

  return (
    <>
      <header className="sticky top-0 left-0 bg-white md:bg-transparent gap-4 z-50 flex flex-wrap md:flex-nowrap items-center justify-between w-full p-4 xl:grid xl:grid-cols-4">
        <Link to="/">
          <img
            src="/images/dango.svg"
            alt="dango.lgoo"
            className="hidden sm:block h-8 order-1 cursor-pointer"
          />
        </Link>
        <CommandBar
          applets={{ all: applets, popular: popularApplets }}
          action={({ path }) => navigate(path)}
        />
        <div className="flex gap-2 items-center justify-end order-2 md:order-3">
          <>
            {import.meta.env.MODE === "development" && !account ? <ConnectButton /> : null}
            <MenuNotifications />
            <MenuAccounts
              manageAction={(account) => navigate(`/accounts/${account.index}`)}
              images={{
                [AccountType.Spot]: "/images/avatars/spot.png",
                [AccountType.Margin]: "/images/avatars/margin.png",
                [AccountType.Safe]: "/images/avatars/safe.png",
              }}
            />
            <MenuConnections />
          </>
        </div>
      </header>
    </>
  );
};
