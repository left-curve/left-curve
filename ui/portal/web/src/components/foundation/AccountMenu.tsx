import { useAccount, useBalances, useConfig, usePrices, useSessionKey } from "@left-curve/store";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import { Sheet } from "react-modal-sheet";
import { useApp } from "~/hooks/useApp";

import { motion } from "framer-motion";

import { m } from "~/paraglide/messages";
import { Modals } from "../modals/RootModal";

import { AccountCard } from "./AccountCard";

import {
  Button,
  EmptyPlaceholder,
  IconAddCross,
  IconButton,
  IconChevronDown,
  IconLeft,
  IconLogOut,
  IconMobile,
  IconSwitch,
  Tabs,
  twMerge,
  useClickAway,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { AnimatePresence } from "framer-motion";
import { AssetCard } from "./AssetCard";

import type { PropsWithChildren } from "react";
import type React from "react";
import type { AnyCoin, WithAmount } from "@left-curve/store/types";
import { useQuery } from "@tanstack/react-query";

const Root: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type AccountMenuProps = {
  backAllowed?: boolean;
};

const Menu: React.FC<AccountMenuProps> = ({ backAllowed }) => {
  const { settings, isSidebarVisible } = useApp();
  const { formatNumberOptions } = settings;
  const { account } = useAccount();
  const { history } = useRouter();
  const [isAccountSelectorActive, setAccountSelectorActive] = useState(false);

  const { data: balances = {} } = useBalances({ address: account?.address });
  const { calculateBalance } = usePrices();

  const totalBalance = calculateBalance(balances, {
    format: true,
    formatOptions: {
      ...formatNumberOptions,
      currency: "USD",
    },
  });

  useEffect(() => {
    if (!isSidebarVisible) setAccountSelectorActive(false);
  }, [isSidebarVisible]);

  if (!account) return null;

  return (
    <>
      <div className="w-full flex items-center flex-col gap-6 relative md:pt-4">
        <div className="flex flex-col w-full items-center gap-5">
          {backAllowed ? (
            <div className="w-full flex gap-2">
              <IconButton variant="link" onClick={() => history.go(-1)}>
                <IconChevronDown className="rotate-90" />
                <span className="h4-bold">{m["common.accounts"]()} </span>
              </IconButton>
            </div>
          ) : null}
          <AnimatePresence mode="wait">
            <motion.div
              className="flex flex-col items-center h-full w-full px-4"
              key={account.address}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.2 }}
              exit={{ opacity: 0 }}
            >
              <AccountCard
                account={account}
                balance={totalBalance}
                isSelectorActive={isAccountSelectorActive}
              />
            </motion.div>
          </AnimatePresence>
        </div>

        <AnimatePresence mode="wait">
          <motion.div
            className="h-full w-full"
            key={isAccountSelectorActive ? "selector" : "assets"}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
            exit={{ opacity: 0 }}
          >
            {isAccountSelectorActive ? (
              <Selector onBack={() => setAccountSelectorActive(!isAccountSelectorActive)} />
            ) : (
              <Assets onSwitch={() => setAccountSelectorActive(!isAccountSelectorActive)} />
            )}
          </motion.div>
        </AnimatePresence>
      </div>
    </>
  );
};

export const Desktop: React.FC = () => {
  const menuRef = useRef<HTMLDivElement>(null);
  const { setSidebarVisibility, isSidebarVisible, isQuestBannerVisible } = useApp();

  useClickAway(menuRef, (e) => {
    if (e.target instanceof HTMLElement && e.target.closest("[dng-connect-button]")) return;
    setSidebarVisibility(false);
  });

  return (
    <div
      ref={menuRef}
      className={twMerge(
        "transition-all lg:absolute fixed top-0 flex h-[92vh] justify-end z-50 duration-300 w-full lg:max-w-[376px] bg-[linear-gradient(90deg,_rgba(0,_0,_0,_0)_3.2%,_rgba(46,_37,_33,_0.1)_19.64%,_rgba(255,_255,_255,_0.1)_93.91%)]",
        isSidebarVisible ? "right-0" : "right-[-50vw]",
        isQuestBannerVisible ? "h-[92vh]" : "h-[100svh]",
      )}
    >
      <div className="lg:pr-2 lg:py-4 w-full relative z-10">
        <div className="w-full bg-white-100 flex flex-col items-center h-full rounded-t-2xl lg:rounded-2xl border border-gray-100">
          <Menu />
        </div>
      </div>
    </div>
  );
};

export const Mobile: React.FC = () => {
  const { isSidebarVisible, setSidebarVisibility } = useApp();

  return (
    <Sheet isOpen={isSidebarVisible} onClose={() => setSidebarVisibility(false)} rootId="root">
      <Sheet.Container className="!bg-white-100 !rounded-t-2xl !shadow-none">
        <Sheet.Header />
        <Sheet.Content>
          <Menu />
        </Sheet.Content>
      </Sheet.Container>
      <Sheet.Backdrop onTap={() => setSidebarVisibility(false)} />
    </Sheet>
  );
};

type AssetsProps = {
  onSwitch: () => void;
};

export const Assets: React.FC<AssetsProps> = ({ onSwitch }) => {
  const { setSidebarVisibility, showModal } = useApp();
  const navigate = useNavigate();
  const { connector } = useAccount();
  const { deleteSessionKey } = useSessionKey();
  const { isMd } = useMediaQuery();
  const [activeTab, setActiveTab] = useState("wallet");

  return (
    <div className="flex flex-col w-full gap-4 items-center">
      <div className="md:self-end flex gap-2 items-center justify-center w-full px-4">
        <Button
          fullWidth
          size="md"
          onClick={() => [
            navigate({ to: "/transfer", search: { action: "receive" } }),
            setSidebarVisibility(false),
          ]}
        >
          <IconAddCross className="w-5 h-5" /> <span> {m["common.fund"]()}</span>
        </Button>
        <Button fullWidth variant="secondary" size="md" onClick={onSwitch}>
          <IconSwitch className="w-5 h-5" /> <span> {m["common.switch"]()}</span>
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
            deleteSessionKey();
          }}
        >
          <IconLogOut />
        </IconButton>
      </div>
      <div className="px-4 py-0 w-full">
        <Tabs
          color="line-red"
          layoutId="tabs-assets-account"
          selectedTab={activeTab}
          keys={["wallet", "vaults", "orders"]}
          fullWidth
          onTabChange={setActiveTab}
        />
      </div>
      <div className="flex flex-col w-full overflow-y-scroll scrollbar-none pb-4 h-full max-h-[calc(100svh-20rem)]">
        <CoinsList type={activeTab as "wallet" | "orders" | "vaults"} />
      </div>
    </div>
  );
};

export const CoinsList: React.FC<{ type: "wallet" | "orders" | "vaults" }> = ({ type }) => {
  const { getCoinInfo } = useConfig();
  const { account } = useAccount();
  const { data: balances = {} } = useBalances({ address: account?.address });

  const allCoins = useMemo(
    () =>
      Object.entries(balances).map(([denom, amount]) =>
        Object.assign(getCoinInfo(denom), { amount }),
      ),
    [balances],
  );

  const walletCoins: WithAmount<AnyCoin>[] = useMemo(() => {
    return allCoins.filter(({ type }) => type === "native");
  }, [allCoins]);

  const vaultCoins: WithAmount<AnyCoin>[] = useMemo(() => {
    return allCoins.filter(({ type }) => type === "lp");
  }, [allCoins]);

  return (
    <motion.div
      key={type}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.5, ease: "easeInOut" }}
    >
      {type === "wallet" ? <WalletTab coins={walletCoins} /> : null}
      {type === "vaults" ? <VaultsTab coins={vaultCoins} /> : null}
      {/* {type === "orders" ? <OrdersTab orders={orders} /> : null} */}
    </motion.div>
  );
};

export const WalletTab: React.FC<{ coins: WithAmount<AnyCoin>[] }> = ({ coins }) => {
  if (!coins || coins.length === 0) {
    return (
      <div className="px-4">
        <EmptyPlaceholder component={m["accountMenu.noWalletCoins"]()} className="p-4" />
      </div>
    );
  }
  return (
    <>
      {coins.map((coin) => (
        <AssetCard key={coin.denom} coin={coin} />
      ))}
    </>
  );
};

export const VaultsTab: React.FC<{ coins: WithAmount<AnyCoin>[] }> = ({ coins }) => {
  if (!coins || coins.length === 0) {
    return (
      <div className="px-4">
        <EmptyPlaceholder component={m["accountMenu.noVaults"]()} className="p-4" />
      </div>
    );
  }
  return (
    <>
      {coins.map((coin) => (
        <AssetCard key={coin.denom} coin={coin} />
      ))}
    </>
  );
};

export const OrdersTab: React.FC<{ orders: any[] }> = ({ orders }) => {
  if (!orders || orders.length === 0) {
    return (
      <div className="px-4">
        <EmptyPlaceholder component={m["dex.protrade.spot.noOpenOrders"]()} className="p-4" />
      </div>
    );
  }
  return (
    <>
      {orders.map((order) => (
        <div key={order.id}>{order.id}</div>
      ))}
    </>
  );
};

type SelectorProps = {
  onBack: () => void;
};

export const Selector: React.FC<SelectorProps> = ({ onBack }) => {
  const { setSidebarVisibility } = useApp();
  const navigate = useNavigate();
  const { account, accounts, changeAccount } = useAccount();

  if (!account) return null;

  return (
    <div className="flex flex-col w-full gap-4 items-center">
      <div className="flex items-center justify-between gap-4 w-full max-w-[22.5rem] md:max-w-[20.5rem]">
        <Button
          size="sm"
          variant="link"
          className="flex justify-center items-center"
          onClick={onBack}
        >
          <IconLeft className="w-[22px] h-[22px]" />
          <span>{m["common.back"]()}</span>
        </Button>
        <Button onClick={() => [setSidebarVisibility(false), navigate({ to: "/account/create" })]}>
          <IconAddCross className="w-5 h-5" /> <span>{m["accountMenu.accounts.addAccount"]()}</span>
        </Button>
      </div>
      <div className="relative w-full h-full">
        <div className="relative flex flex-col items-center w-full overflow-scroll gap-4 scrollbar-none pb-[7rem] pt-2 max-h-[52svh] md:max-h-[68vh]">
          {accounts
            ?.filter((acc) => acc.address !== account.address)
            .sort((a, b) => a.index - b.index)
            .map((account) => (
              <AccountCard.Preview
                key={account.address}
                account={account}
                onAccountSelect={(acc) => changeAccount?.(acc.address)}
              />
            ))}
        </div>
        <div className="absolute h-2 w-full bottom-[1.5rem] z-50 max-w-[22.5rem] md:max-w-[20.5rem] pointer-events-none left-1/2 -translate-x-1/2">
          <div className="bg-gradient-to-b from-transparent from-20% to-white-100 h-[3rem] w-full rounded-b-2xl" />
        </div>
      </div>
    </div>
  );
};

const ExportComponent = Object.assign(Root, {
  Desktop,
  Mobile,
  Assets,
  Selector,
});

export { ExportComponent as AccountMenu };
