import { CommandBar, MenuAccounts, MenuConnections, MenuNotifications } from "@dango/shared";

import { AccountType } from "@leftcurve/types";

import { useAccount } from "@leftcurve/react";
import { Link, useNavigate } from "react-router-dom";
import { applets } from "../../applets";

export const Header: React.FC = () => {
  const navigate = useNavigate();
  const { account } = useAccount();

  return (
    <>
      <header className="sticky top-0 left-0 bg-white lg:bg-transparent gap-4 z-50 flex flex-wrap lg:flex-nowrap items-center justify-between w-full p-4 xl:grid xl:grid-cols-4">
        <Link to="/" className="w-fit">
          <img
            src="/images/dango.svg"
            alt="dango logo"
            className="hidden sm:block h-8 order-1 cursor-pointer"
          />
        </Link>
        <CommandBar applets={applets} action={({ path }) => navigate(path)} />
        <div className="flex gap-2 items-center justify-end order-2 lg:order-3">
          <MenuNotifications />
          <MenuAccounts
            manageAction={(account) => navigate(`/accounts?address=${account.address}`)}
            createAction={() => navigate("/account-creation")}
            images={{
              [AccountType.Spot]: "/images/avatars/spot.svg",
              [AccountType.Margin]: "/images/avatars/margin.svg",
              [AccountType.Safe]: "/images/avatars/safe.svg",
            }}
          />
          <MenuConnections />
        </div>
      </header>
    </>
  );
};
