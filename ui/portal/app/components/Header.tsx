"use client";

import { CommandBar, MenuAccounts, MenuConnections, MenuNotifications } from "@dango/shared";

import { AccountType } from "@leftcurve/types";

import { useAccount } from "@leftcurve/react";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { applets } from "../../applets";

export const Header: React.FC = () => {
  const { push: navigate } = useRouter();
  const { account } = useAccount();

  return (
    <>
      <header className="sticky top-0 left-0 bg-white md:bg-transparent gap-4 z-50 flex flex-wrap md:flex-nowrap items-center justify-between w-full p-4 xl:grid xl:grid-cols-4">
        <Link href="/" className="w-fit">
          <img
            src="/images/dango.svg"
            alt="dango logo"
            className="hidden sm:block h-8 order-1 cursor-pointer"
          />
        </Link>
        <CommandBar applets={applets} action={({ path }) => navigate(path)} />
        <div className="flex gap-2 items-center justify-end order-2 md:order-3">
          <MenuNotifications />
          <MenuAccounts
            manageAction={(account) => navigate(`/accounts/${account.index}`)}
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
