import {
  useAccount,
  useBalances,
  useOrdersByUser,
  usePrices,
  useSessionKey,
} from "@left-curve/store";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { Sheet } from "react-modal-sheet";
import { useApp } from "~/hooks/useApp";

import { motion } from "framer-motion";

import { m } from "~/paraglide/messages";
import { Modals } from "../modals/RootModal";

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
} from "@left-curve/applets-kit";
import { AnimatePresence } from "framer-motion";
import { AccountCard } from "./AccountCard";
import { AssetCard } from "./AssetCard";
import { EmptyPlaceholder } from "./EmptyPlaceholder";

import type { PropsWithChildren } from "react";
import type React from "react";

import { Notifications } from "../notifications/Notifications";
import { useNotifications } from "~/hooks/useNotifications";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
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
    <div className="w-full flex items-center flex-col gap-6 relative md:pt-4">
      <div className="flex flex-col w-full items-center gap-5">
        {backAllowed ? (
          <div className="w-full flex gap-2">
            <IconButton variant="link" onClick={() => history.go(-1)}>
              <IconChevronDown className="rotate-90" />
              <span className="h4-bold text-primary-900">{m["common.accounts"]()} </span>
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
  );
};

const Desktop: React.FC = () => {
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
        <div className="w-full bg-surface-primary-rice flex flex-col items-center h-full rounded-t-2xl lg:rounded-2xl border border-secondary-gray">
          <Menu />
        </div>
      </div>
    </div>
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
    <div className="flex flex-col w-full gap-6 items-center">
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
          keys={["wallet", "activities"]}
          fullWidth
          onTabChange={setActiveTab}
        />
      </div>
      <motion.div
        key={activeTab}
        className="flex flex-col w-full overflow-y-scroll scrollbar-none pb-4 h-full max-h-[calc(100svh-20rem)]"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.5, ease: "easeInOut" }}
      >
        {activeTab === "wallet" ? <WalletTab /> : null}
        {activeTab === "activities" ? <NotificationsTab /> : null}
      </motion.div>
    </div>
  );
};

export const WalletTab: React.FC = () => {
  const { account } = useAccount();
  const { data: balances = {} } = useBalances({ address: account?.address });

  const { data: orders = [] } = useOrdersByUser();

  const allBalances = Object.entries(balances);

  return (
    <div className="flex flex-col w-full items-center">
      {allBalances.length > 0 ? (
        <div className="flex flex-col w-full gap-4 items-center">
          <div className="flex flex-col w-full gap-2">
            {allBalances.map(([denom, amount]) => (
              <AssetCard key={denom} coin={{ denom, amount }} />
            ))}
          </div>
        </div>
      ) : (
        <div className="px-4">
          <EmptyPlaceholder component={m["accountMenu.noWalletCoins"]()} className="p-4" />
        </div>
      )}
    </div>
  );
};

export const NotificationsTab: React.FC = () => {
  const navigate = useNavigate();
  const { setSidebarVisibility } = useApp();

  const { totalNotifications } = useNotifications({});

  return (
    <div className="pb-[2.5rem] flex flex-col">
      <Notifications
        className="max-h-[41rem] overflow-y-scroll scrollbar-none"
        maxNotifications={5}
      />
      {totalNotifications > 0 ? (
        <div className="p-4 flex items-center justify-center">
          <Button
            variant="link"
            className="py-0 h-fit"
            onClick={() => [navigate({ to: "/notifications" }), setSidebarVisibility(false)]}
          >
            {m["common.viewAll"]()}
          </Button>
        </div>
      ) : null}
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
          <div className="bg-gradient-to-b from-transparent from-20% to-bg-primary-rice h-[3rem] w-full rounded-b-2xl" />
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
