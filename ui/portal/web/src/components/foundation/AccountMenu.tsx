import {
  useAccount,
  useBalances,
  useOrdersByUser,
  usePrices,
  useSessionKey,
} from "@left-curve/store";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import { Sheet } from "react-modal-sheet";

import { motion } from "framer-motion";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  Button,
  IconAddCross,
  IconLeft,
  Tabs,
  IconSwitch,
  twMerge,
  useClickAway,
  useMediaQuery,
  IconButton,
  IconChevronDown,
  IconMobile,
  IconLogOut,
  createContext,
  useApp,
  Modals,
  useHeaderHeight,
  useBodyScrollLock,
} from "@left-curve/applets-kit";
import { AnimatePresence } from "framer-motion";
import { AccountCard } from "./AccountCard";
import { AssetCard } from "./AssetCard";
import { EmptyPlaceholder } from "./EmptyPlaceholder";
import { Activities } from "../activities/Activities";

import { Direction } from "@left-curve/dango/types";

import type React from "react";
import type { Coins } from "@left-curve/dango/types";
import { Decimal } from "@left-curve/dango/utils";

const [AccountMenuProvider, useAccountMenu] = createContext<{
  balances: Coins;
  totalBalance: string;
}>();

const Container: React.FC = () => {
  const { settings } = useApp();
  const { isLg } = useMediaQuery();
  const { account } = useAccount();
  const { calculateBalance } = usePrices();

  const { formatNumberOptions } = settings;

  const { data: balances = {} } = useBalances({ address: account?.address });

  const { data: orders = [] } = useOrdersByUser();

  const allBalances = useMemo(() => {
    if (!orders.length) return balances;
    return orders.reduce(
      (acc, order) => {
        const { baseDenom, quoteDenom, amount, direction, remaining } = order;
        if (direction === Direction.Buy) {
          const quoteAmount = remaining;
          acc[quoteDenom] = Decimal(acc[quoteDenom] || "0")
            .plus(quoteAmount)
            .toFixed();
          const restAmount = Decimal(amount).minus(remaining).toFixed();
          acc[baseDenom] = Decimal(acc[baseDenom] || "0")
            .minus(restAmount)
            .toFixed();
        } else {
          const baseAmount = remaining;
          acc[baseDenom] = Decimal(acc[baseDenom] || "0")
            .plus(baseAmount)
            .toFixed();
          const restAmount = Decimal(amount).minus(remaining).toFixed();
          acc[quoteDenom] = Decimal(acc[quoteDenom] || "0")
            .minus(restAmount)
            .toFixed();
        }
        return acc;
      },
      { ...balances },
    );
  }, [balances, orders]);

  const totalBalance = useMemo(
    () =>
      calculateBalance(allBalances, {
        format: true,
        formatOptions: {
          ...formatNumberOptions,
          currency: "USD",
        },
      }),
    [allBalances],
  );

  return (
    <AccountMenuProvider value={{ balances: allBalances, totalBalance }}>
      <AnimatePresence>{isLg ? <Desktop /> : <Mobile />}</AnimatePresence>
    </AccountMenuProvider>
  );
};

type AccountMenuProps = {
  backAllowed?: boolean;
};

const Menu: React.FC<AccountMenuProps> = ({ backAllowed }) => {
  const { isSidebarVisible } = useApp();
  const { account, isUserActive } = useAccount();
  const { history } = useRouter();
  const { totalBalance } = useAccountMenu();
  const [isAccountSelectorActive, setAccountSelectorActive] = useState(false);

  useEffect(() => {
    if (!isSidebarVisible) setAccountSelectorActive(false);
  }, [isSidebarVisible]);

  if (!account) return null;

  return (
    <div className="w-full flex items-center flex-col gap-6 relative md:pt-4 flex-1 h-full">
      <div className="flex flex-col w-full items-center gap-5">
        {backAllowed ? (
          <div className="w-full flex gap-2">
            <IconButton variant="link" onClick={() => history.go(-1)}>
              <IconChevronDown className="rotate-90" />
              <span className="h4-bold text-ink-primary-900">{m["common.accounts"]()} </span>
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
              isUserActive={isUserActive}
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
  );
};

const Desktop: React.FC = () => {
  const menuRef = useRef<HTMLDivElement>(null);
  const { setSidebarVisibility, isSidebarVisible, modal } = useApp();
  const headerHeight = useHeaderHeight();
  useBodyScrollLock(isSidebarVisible);

  useClickAway(menuRef, (e) => {
    if (
      (e.target instanceof HTMLElement && e.target.closest("[dng-connect-button]")) ||
      modal.modal
    )
      return;
    setSidebarVisibility(false);
  });

  return (
    <AnimatePresence>
      {isSidebarVisible && (
        <motion.div
          ref={menuRef}
          key="desktop-sidebar"
          initial={{ opacity: 0, x: "50%" }}
          animate={{ opacity: 1, x: 0 }}
          exit={{ opacity: 0, x: "50%" }}
          transition={{ duration: 0.25, ease: "easeInOut" }}
          className={twMerge(
            "fixed bottom-0 right-0 flex h-[100vh] justify-end z-50 w-full lg:max-w-[376px]",
          )}
          style={{
            height: `calc(100% - ${headerHeight - 8 || 60}px)`,
          }}
        >
          <div className="lg:pr-2 lg:py-2 w-full relative z-10 flex items-end">
            <div className="h-full w-full bg-surface-primary-rice flex flex-col items-center rounded-t-2xl lg:rounded-2xl border border-outline-secondary-gray overflow-hidden">
              <Menu />
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};

const Mobile: React.FC = () => {
  const { isSidebarVisible, setSidebarVisibility } = useApp();

  return (
    <Sheet isOpen={isSidebarVisible} onClose={() => setSidebarVisibility(false)} rootId="root">
      <Sheet.Container className="!bg-surface-primary-rice !rounded-t-2xl !shadow-none">
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

const Assets: React.FC<AssetsProps> = ({ onSwitch }) => {
  const { setSidebarVisibility, showModal } = useApp();
  const navigate = useNavigate();
  const { connector } = useAccount();
  const { deleteSessionKey } = useSessionKey();
  const { isMd } = useMediaQuery();
  const [activeTab, setActiveTab] = useState("wallet");

  return (
    <div className="flex flex-col w-full gap-6 items-center h-full">
      <div className="md:self-end flex gap-2 items-center justify-center w-full px-4">
        <Button
          fullWidth
          size="md"
          onClick={() => [navigate({ to: "/bridge" }), setSidebarVisibility(false)]}
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
          keys={["wallet", "activities"]}
          fullWidth
          onTabChange={setActiveTab}
        />
      </div>
      <motion.div
        key={activeTab}
        className="flex flex-col w-full overflow-hidden overflow-y-scroll scrollbar-none pb-4 h-full"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.5, ease: "easeInOut" }}
      >
        {activeTab === "wallet" ? <WalletTab /> : null}
        {activeTab === "activities" ? <ActivityTab /> : null}
      </motion.div>
    </div>
  );
};

export const WalletTab: React.FC = () => {
  const context = useAccountMenu();
  const balances = Object.entries(context.balances);
  const { calculateBalance } = usePrices();

  const sortedBalances = useMemo(() => {
    return balances.sort(([denomA, amountA], [denomB, amountB]) => {
      const usdA = Number(calculateBalance({ [denomA]: amountA }, { format: false }) ?? 0);
      const usdB = Number(calculateBalance({ [denomB]: amountB }, { format: false }) ?? 0);
      if (usdB !== usdA) return usdB - usdA;
      return denomA.localeCompare(denomB);
    });
  }, [balances, calculateBalance]);

  return (
    <div className="flex flex-col w-full items-center max-h-full overflow-hidden overflow-y-scroll scrollbar-none">
      {sortedBalances.length > 0 ? (
        sortedBalances.map(([denom, amount]) => <AssetCard key={denom} coin={{ denom, amount }} />)
      ) : (
        <div className="px-4">
          <EmptyPlaceholder component={m["accountMenu.noWalletCoins"]()} className="p-4" />
        </div>
      )}
    </div>
  );
};

export const ActivityTab: React.FC = () => {
  return (
    <div className="flex flex-col pb-[12rem]">
      <Activities className="overflow-y-scroll scrollbar-none" />
    </div>
  );
};

type SelectorProps = {
  onBack: () => void;
};

const Selector: React.FC<SelectorProps> = ({ onBack }) => {
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
          <div className="bg-gradient-to-b from-transparent from-20% to-bg-surface-primary-rice h-[3rem] w-full rounded-b-2xl" />
        </div>
      </div>
    </div>
  );
};

export const AccountMenu = Object.assign(Container, {
  Desktop,
  Mobile,
  Assets,
  Selector,
});
