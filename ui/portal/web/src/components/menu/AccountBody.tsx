import { AccountCard, Button, twMerge } from "@left-curve/applets-kit";
import { useAccount, useBalances, usePrices } from "@left-curve/store-react";
import { motion } from "framer-motion";
import type React from "react";
import { useState } from "react";
import { useApp } from "~/hooks/useApp";
import { AssetTab } from "./AssetTab";

import { useNavigate } from "@tanstack/react-router";

export const AccountMenuBody: React.FC = () => {
  const navigate = useNavigate();
  const { setSidebarVisibility } = useApp();
  const { account, connector } = useAccount();
  const [menuAccountActiveLink, setMenuAccountActiveLink] = useState<"Assets" | "Earn" | "Pools">(
    "Assets",
  );

  const { data: balances = {} } = useBalances({ address: account?.address });
  const { calculateBalance } = usePrices();

  const totalBalance = calculateBalance(balances, { format: true });

  if (!account) return null;

  return (
    <>
      <div className="p-4 w-full flex items-center flex-col gap-5">
        <AccountCard
          account={account}
          balance={totalBalance}
          logout={() => {
            setSidebarVisibility(false);
            connector?.disconnect();
          }}
        />
        <div className="md:self-end flex gap-4 items-center justify-center w-full">
          <Button
            fullWidth
            size="md"
            onClick={() => [
              navigate({ to: "/send-and-receive", search: { action: "receive" } }),
              setSidebarVisibility(false),
            ]}
          >
            Fund
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
            Send
          </Button>
        </div>

        <motion.ul className="flex gap-4 text-base relative border-b border-b-gray-100 w-full items-center">
          {Array.from(["Assets", "Earn", "Pools"]).map((tab) => {
            const isActive = tab === menuAccountActiveLink;
            return (
              <motion.li
                className="relative px-4 transition-all flex-1 flex items-center justify-center py-3 cursor-pointer"
                key={`navLink-${tab}`}
                onClick={() => setMenuAccountActiveLink(tab as "Assets" | "Earn" | "Pools")}
              >
                <p
                  className={twMerge(
                    "italic font-medium font-exposure transition-all",
                    isActive ? "text-red-bean-400" : "text-gray-300",
                  )}
                >
                  {tab}
                </p>
                {isActive ? (
                  <motion.div
                    className="w-full h-[1px] bg-red-bean-400 absolute bottom-0 left-0"
                    layoutId="underline"
                  />
                ) : null}
              </motion.li>
            );
          })}
        </motion.ul>
      </div>

      {menuAccountActiveLink === "Assets" ? <AssetTab /> : null}
    </>
  );
};
