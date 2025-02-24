import {
  AccountCard,
  Button,
  IconDoubleChevronRight,
  twMerge,
  useClickAway,
} from "@left-curve/applets-kit";
import type React from "react";
import { useRef, useState } from "react";

import { useAccount, useBalances, usePrices } from "@left-curve/store-react";
import { motion } from "framer-motion";
import { useApp } from "~/hooks/useApp";
import { AssetTab } from "./AssetTab";

export const AccountMenu: React.FC = () => {
  const { setSidebarVisibility, isSidebarVisible } = useApp();
  const { account, connector } = useAccount();
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuAccountActiveLink, setMenuAccountActiveLink] = useState<"Assets" | "Earn" | "Pools">(
    "Assets",
  );

  useClickAway(menuRef, () => setSidebarVisibility(false));

  const { data: balances = {} } = useBalances({ address: account?.address });
  const { calculateBalance } = usePrices();

  const totalBalance = calculateBalance(balances, { format: true });

  if (!account) return null;

  return (
    <div
      ref={menuRef}
      className={twMerge(
        "transition-all lg:absolute fixed pt-4 top-0 flex h-[100vh] justify-end z-50 duration-300 delay-100 w-full lg:max-w-[422px] bg-[linear-gradient(90deg,_rgba(0,_0,_0,_0)_3.2%,_rgba(46,_37,_33,_0.1)_19.64%,_rgba(255,_255,_255,_0.1)_93.91%)]",
        isSidebarVisible ? "right-0" : "right-[-100vh]",
      )}
    >
      <div
        className="hidden group h-full py-4 lg:flex justify-end w-[84px] mr-[-20px]"
        onClick={() => setSidebarVisibility(false)}
      >
        <div className="h-full py-2 pr-8 group-hover:translate-x-2 pl-2 text-gray-500 cursor-pointer group-hover:bg-gray-300/20 rounded-tl-lg rounded-bl-lg transition-all">
          <IconDoubleChevronRight className="transition-all group-hover:scale-90" />
        </div>
      </div>
      <div className="lg:pr-2 lg:py-4 w-full relative z-10">
        <div className="w-full bg-white-100 flex flex-col items-center h-full rounded-t-2xl lg:rounded-2xl border border-gray-100">
          <div className="h-[24x] w-full flex items-center">
            <span className="w-8 h-1 bg-gray-100 rounded-md" />
          </div>

          <div className="p-4 w-full flex items-center flex-col gap-5">
            <AccountCard
              account={account}
              balance={totalBalance}
              logout={() => connector?.disconnect()}
            />
            <div className="md:self-end flex gap-4 items-center justify-center w-full">
              <Button fullWidth size="md">
                Fund
              </Button>
              <Button fullWidth variant="secondary" size="md">
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
        </div>
      </div>
    </div>
  );
};
