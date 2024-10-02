import { CommandBar, MenuAccounts, MenuConnections, MenuNotifications } from "@leftcurve/ui";

import { AccountType } from "@leftcurve/types";

import { useNavigate } from "react-router-dom";
import { applets, popularApplets } from "../../applets";

export const Header: React.FC = () => {
  const navigate = useNavigate();

  return (
    <>
      <header className="fixed top-0 left-0 bg-white md:bg-transparent gap-4 z-50 flex flex-wrap md:flex-nowrap items-center justify-between w-full p-4 xl:grid xl:grid-cols-4">
        <div className="h-10 w-10 rounded-full bg-gray-200 order-1" />
        <CommandBar
          applets={{ all: applets, popular: popularApplets }}
          action={({ path }) => navigate(path)}
        />
        <div className="flex gap-2 items-center justify-end order-2 md:order-3">
          <>
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
