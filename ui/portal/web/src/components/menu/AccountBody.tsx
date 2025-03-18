import {
  AccountCard,
  Button,
  IconAddCross,
  IconButton,
  IconChevronLeft,
  IconLogOut,
  IconMobile,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { useAccount, useBalances, usePrices } from "@left-curve/store-react";
import type React from "react";
import { useState } from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { AccountTab } from "./AccountTab";
import { AssetTab } from "./AssetTab";

import { useNavigate } from "@tanstack/react-router";
import { Modals } from "../Modal";

export const AccountMenuBody: React.FC = () => {
  const navigate = useNavigate();
  const [tab, setTab] = useState<"account" | "assets">("assets");
  const { setSidebarVisibility, showModal, formatNumberOptions } = useApp();
  const { account, connector } = useAccount();
  const { isMd } = useMediaQuery();

  const { data: balances = {} } = useBalances({ address: account?.address });
  const { calculateBalance } = usePrices();

  const totalBalance = calculateBalance(balances, {
    format: true,
    formatOptions: formatNumberOptions,
  });

  if (!account) return null;

  return (
    <>
      <div className="p-4 w-full flex items-center flex-col gap-5 relative">
        <AccountCard account={account} balance={totalBalance} />
        <IconButton
          className="absolute top-8 right-8 z-30"
          size="sm"
          variant="secondary"
          onClick={() => setTab(tab === "account" ? "assets" : "account")}
        >
          <IconChevronLeft className="w-4 h-4 -rotate-90" />
        </IconButton>
        {tab === "assets" && (
          <div className="md:self-end flex gap-2 items-center justify-center w-full">
            <Button
              fullWidth
              size="md"
              onClick={() => [
                navigate({ to: "/send-and-receive", search: { action: "receive" } }),
                setSidebarVisibility(false),
              ]}
            >
              {m["common.funds"]()}
            </Button>
            <Button
              fullWidth
              variant="secondary"
              size="md"
              onClick={() => [
                navigate({ to: "/send-and-receive", search: { action: "send" } }),
                setSidebarVisibility(false),
              ]}
            >
              {m["common.send"]()}
            </Button>
            {isMd ? (
              <IconButton variant="secondary" onClick={() => showModal(Modals.QRConnect)}>
                <IconMobile />
              </IconButton>
            ) : null}
            <IconButton
              variant="secondary"
              onClick={() => {
                setSidebarVisibility(false);
                connector?.disconnect();
              }}
            >
              <IconLogOut />
            </IconButton>
          </div>
        )}
        {tab === "account" && (
          <div className="flex items-center justify-between gap-4 w-full">
            <p>{m["accountMenu.accounts.otherAccounts"]()}</p>
            <Button
              onClick={() => [setSidebarVisibility(false), navigate({ to: "/create-account" })]}
            >
              <IconAddCross className="w-5 h-5" />{" "}
              <span>{m["accountMenu.accounts.addAccount"]()}</span>
            </Button>
          </div>
        )}
      </div>

      {tab === "assets" && <AssetTab />}
      {tab === "account" && <AccountTab />}
    </>
  );
};
