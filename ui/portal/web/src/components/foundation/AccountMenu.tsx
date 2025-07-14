import {
  useAccount,
  useBalances,
  useConfig,
  useOrdersByUser,
  usePrices,
  usePublicClient,
  useSessionKey,
} from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import { Sheet } from "react-modal-sheet";
import { useApp } from "~/hooks/useApp";

import { motion } from "framer-motion";

import { Decimal, formatNumber, formatUnits } from "@left-curve/dango/utils";
import { m } from "~/paraglide/messages";
import { Modals } from "../modals/RootModal";

import {
  Badge,
  Button,
  IconAddCross,
  IconButton,
  IconChevronDown,
  IconChevronDownFill,
  IconLeft,
  IconLogOut,
  IconMobile,
  IconSwitch,
  PairAssets,
  Tabs,
  twMerge,
  useClickAway,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { Direction } from "@left-curve/dango/types";
import { AnimatePresence } from "framer-motion";
import { AccountCard } from "./AccountCard";
import { AssetCard } from "./AssetCard";
import { EmptyPlaceholder } from "./EmptyPlaceholder";

import type { OrdersByUserResponse, WithId } from "@left-curve/dango/types";
import type { LpCoin, NativeCoin, WithAmount } from "@left-curve/store/types";
import type { PropsWithChildren } from "react";
import type React from "react";

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

const CoinsList: React.FC<{ type: "wallet" | "orders" | "vaults" }> = ({ type }) => {
  const { getCoinInfo } = useConfig();
  const { account } = useAccount();
  const { data: balances = {} } = useBalances({ address: account?.address });

  const { data: orders = [] } = useOrdersByUser();

  const allCoins = useMemo(
    () =>
      Object.entries(balances).map(([denom, amount]) =>
        Object.assign(getCoinInfo(denom), { amount }),
      ),
    [balances],
  );

  const walletCoins = useMemo(() => {
    return allCoins.filter(({ type }) => type === "native") as WithAmount<NativeCoin>[];
  }, [allCoins]);

  const vaultCoins = useMemo(() => {
    return allCoins.filter(({ type }) => type === "lp") as WithAmount<LpCoin>[];
  }, [allCoins]);

  return (
    <motion.div
      key={type}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.5, ease: "easeInOut" }}
    >
      {type === "wallet" ? <WalletTab coins={walletCoins} /> : null}
      {type === "vaults" ? <VaultsTab coins={vaultCoins} /> : null}
      {type === "orders" ? <OrdersTab orders={orders} /> : null}
    </motion.div>
  );
};

export const WalletTab: React.FC<{ coins: WithAmount<NativeCoin>[] }> = ({ coins }) => {
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

const VaultsTab: React.FC<{ coins: WithAmount<LpCoin>[] }> = ({ coins }) => {
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
        <VaultCard key={coin.denom} coin={coin} />
      ))}
    </>
  );
};

const OrdersTab: React.FC<{ orders: WithId<OrdersByUserResponse>[] }> = ({ orders }) => {
  const { coins } = useConfig();
  if (!orders || orders.length === 0) {
    return (
      <div className="px-4">
        <EmptyPlaceholder component={m["dex.protrade.spot.noOpenOrders"]()} className="p-4" />
      </div>
    );
  }

  const ordersGoupedByPair = useMemo(() => {
    return orders.reduce(
      (acc, order) => {
        const base = coins[order.baseDenom];
        const quote = coins[order.quoteDenom];
        const pairId = `${base.symbol}-${quote.symbol}`;
        if (!acc[pairId]) acc[pairId] = [];
        acc[pairId].push(order);
        return acc;
      },
      {} as Record<string, WithId<OrdersByUserResponse>[]>,
    );
  }, [orders]);

  return (
    <>
      {Object.entries(ordersGoupedByPair).map(([pairId, orders]) => (
        <OrderCard key={pairId} pairId={pairId} orders={orders} />
      ))}
    </>
  );
};

type VaultCardProps = {
  coin: WithAmount<LpCoin>;
};

const VaultCard: React.FC<VaultCardProps> = ({ coin }) => {
  const publicClient = usePublicClient();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const [isExpanded, setIsExpanded] = useState<boolean>(false);

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  const userLiquidity = useQuery({
    queryKey: ["lpAmounts", coin.symbol],
    queryFn: async () => {
      const [{ amount: baseAmount }, { amount: quoteAmount }] =
        await publicClient.simulateWithdrawLiquidity({
          baseDenom: coin.base.denom,
          quoteDenom: coin.quote.denom,
          lpBurnAmount: coin.amount,
        });
      const baseParseAmount = formatUnits(baseAmount, coin.base.decimals);
      const quoteParseAmount = formatUnits(quoteAmount, coin.quote.decimals);

      return {
        innerBase: baseParseAmount,
        innerQuote: quoteParseAmount,
      };
    },
  });

  const assets = [coin.base, coin.quote];

  const basePrice = getPrice(userLiquidity.data?.innerBase || "0", coin.base.denom);
  const quotePrice = getPrice(userLiquidity?.data?.innerQuote || "0", coin.quote.denom);

  const totalPrice = formatNumber(Decimal(basePrice).plus(quotePrice).toString(), {
    ...formatNumberOptions,
    currency: "USD",
  });

  return (
    <motion.div
      layout="position"
      className="flex flex-col p-4 hover:bg-surface-tertiary-rice hover:cursor-pointer"
      onClick={() => setIsExpanded(!isExpanded)}
    >
      <div
        className={twMerge("flex items-center justify-between transition-all", {
          "pb-2": isExpanded,
        })}
      >
        <div className="flex gap-2 items-center">
          <div className="flex h-8 w-12">
            <PairAssets assets={assets} />
          </div>
          <div className="flex flex-col">
            <p className="text-primary-900 diatype-m-bold">{coin.symbol} LP</p>
            <p className="text-tertiary-500 diatype-m-regular">{coin.amount}</p>
          </div>
        </div>
        <div className="flex flex-col items-end">
          <p className="text-primary-900 diatype-m-bold">{totalPrice}</p>
          <IconChevronDownFill
            className={twMerge("w-4 h-4 text-gray-200 transition-all", {
              "rotate-180": isExpanded,
            })}
          />
        </div>
      </div>
      <AnimatePresence initial={false}>
        {isExpanded && (
          <motion.div
            layout="position"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: "easeInOut" }}
            className="overflow-hidden flex flex-col gap-2 pl-14 w-full"
          >
            {[
              { ...coin.base, amount: userLiquidity.data?.innerBase },
              { ...coin.quote, amount: userLiquidity.data?.innerQuote },
            ].map((asset, index) => (
              <div
                key={`${asset.denom}-${index}`}
                className="flex items-center justify-between text-tertiary-500 diatype-m-regular"
              >
                <p className="flex items-center gap-2">{asset.symbol}</p>
                <p>{formatNumber(asset.amount || "0", { ...formatNumberOptions })}</p>
              </div>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
};

type OrderCardProps = {
  pairId: string;
  orders: WithId<OrdersByUserResponse>[];
};

const OrderCard: React.FC<OrderCardProps> = ({ pairId, orders }) => {
  const { coins } = useConfig();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const [isExpanded, setIsExpanded] = useState<boolean>(false);

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  const total = useMemo(
    () => orders.reduce((sum, order) => sum.plus(order.remaining), Decimal.ZERO),
    [orders],
  );
  const { baseDenom, quoteDenom } = orders[0];

  const base = coins[baseDenom];

  return (
    <motion.div
      layout="position"
      className="flex flex-col p-4 hover:bg-surface-tertiary-rice hover:cursor-pointer"
      onClick={() => setIsExpanded(!isExpanded)}
    >
      <div
        className={twMerge("flex items-center justify-between transition-all", {
          "pb-2": isExpanded,
        })}
      >
        <div className="flex gap-2 items-center">
          <div className="flex h-8 w-12">
            <PairAssets assets={[base, coins[quoteDenom]]} />
          </div>
          <div className="flex flex-col">
            <p className="text-primary-900 diatype-m-bold">{pairId}</p>
            <p className="text-tertiary-500 diatype-m-regular">
              {getPrice(formatUnits(total.toNumber(), base.decimals), base.denom, { format: true })}
            </p>
          </div>
        </div>
        <div className="flex flex-col items-end">
          <IconChevronDownFill
            className={twMerge("w-4 h-4 text-gray-200 transition-all", {
              "rotate-180": isExpanded,
            })}
          />
        </div>
      </div>
      <AnimatePresence initial={false}>
        {isExpanded && (
          <motion.div
            layout="position"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: "easeInOut" }}
            className="overflow-hidden flex flex-col gap-2 pl-14 w-full"
          >
            {orders.map((order) => {
              return (
                <div
                  key={order.id}
                  className={twMerge(
                    "flex items-center justify-between text-tertiary-500 diatype-m-regular",
                  )}
                >
                  <p className="flex items-center gap-2">
                    <Badge
                      text={m["dex.protrade.spot.direction"]({ direction: order.direction })}
                      color={order.direction === Direction.Buy ? "green" : "red"}
                    />
                  </p>
                  <p>
                    {getPrice(formatUnits(order.remaining, base.decimals), base.denom, {
                      format: true,
                    })}
                  </p>
                </div>
              );
            })}
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
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
