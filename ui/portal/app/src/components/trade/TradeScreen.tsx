import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  Modals,
  useApp,
} from "@left-curve/foundation";
import {
  Decimal,
  formatNumber,
  formatUnits,
} from "@left-curve/dango/utils";
import {
  useAccount,
  useAppConfig,
  useConfig,
  useLiquidityDepthState,
  useLiveTradesState,
  useOrderBookState,
  useProTradeState,
  usePublicClient,
} from "@left-curve/store";
import { useLocalSearchParams, useRouter } from "expo-router";
import { useEffect, useMemo, useState } from "react";
import {
  FlatList,
  Modal,
  Pressable,
  ScrollView,
  TextInput,
  View,
} from "react-native";
import WebView from "react-native-webview";

import { normalizeTradeParams } from "~/features/trade/params";
import {
  Button,
  GlobalText,
  Input,
  MobileTitle,
  ShadowContainer,
  Tabs,
} from "~/components/foundation";
import { PORTAL_WEB_ORIGIN } from "~/constants";
import { useTheme } from "~/hooks/useTheme";

import type { PairId } from "@left-curve/dango/types";

type TradeParams = {
  pairSymbols?: string | string[];
  action?: string | string[];
  order_type?: string | string[];
};

const DEFAULT_PAIR = "ETH-USDC";

export const TradeScreen: React.FC = () => {
  const router = useRouter();
  const params = useLocalSearchParams<TradeParams>();
  const { isConnected } = useAccount();
  const { coins } = useConfig();
  const publicClient = usePublicClient();
  const { showModal, settings } = useApp();

  const [pairSheetVisible, setPairSheetVisible] = useState(false);

  const normalized = useMemo(() => normalizeTradeParams(params, coins), [params, coins]);

  useEffect(() => {
    if (!normalized.changed) return;
    router.setParams({
      pairSymbols: normalized.pairSymbols,
      action: normalized.action,
      order_type: normalized.orderType,
    });
  }, [normalized, router]);

  const pairId = useMemo<PairId>(
    () => ({
      baseDenom: normalized.pair.baseDenom,
      quoteDenom: normalized.pair.quoteDenom,
    }),
    [normalized.pair.baseDenom, normalized.pair.quoteDenom],
  );

  useEffect(() => {
    if (!pairId.baseDenom || !pairId.quoteDenom) return;

    let cancelled = false;

    void publicClient
      ?.getPair({ baseDenom: pairId.baseDenom, quoteDenom: pairId.quoteDenom })
      .catch(() => null)
      .then((pair) => {
        if (cancelled || pair) return;
        router.replace(`/trade/${DEFAULT_PAIR}`);
      });

    return () => {
      cancelled = true;
    };
  }, [pairId.baseDenom, pairId.quoteDenom, publicClient, router]);

  const [inputs, setInputs] = useState<Record<string, { value: string }>>({
    size: { value: "0" },
    price: { value: "0" },
  });

  const controllers = useMemo(
    () => ({
      inputs,
      reset: () => setInputs({ size: { value: "0" }, price: { value: "0" } }),
      setValue: (name: string, value: string) =>
        setInputs((prev) => ({
          ...prev,
          [name]: { value },
        })),
    }),
    [inputs],
  );

  const state = useProTradeState({
    m,
    action: normalized.action,
    onChangeAction: (action) => {
      router.setParams({ action, order_type: state.operation, pairSymbols: normalized.pairSymbols });
    },
    orderType: normalized.orderType,
    onChangeOrderType: (orderType) => {
      router.setParams({ action: state.action, order_type: orderType, pairSymbols: normalized.pairSymbols });
    },
    pairId,
    onChangePairId: (nextPairId) => {
      const baseSymbol = coins.byDenom[nextPairId.baseDenom]?.symbol;
      const quoteSymbol = coins.byDenom[nextPairId.quoteDenom]?.symbol;
      if (!baseSymbol || !quoteSymbol) return;
      router.replace(`/trade/${baseSymbol}-${quoteSymbol}?action=${state.action}&order_type=${state.operation}`);
    },
    bucketRecords: 12,
    controllers,
    submission: {
      onError: () => null,
    },
  });

  const { orderBookStore } = useOrderBookState({ pairId, subscribe: true });
  const currentPrice = orderBookStore((s) => s.currentPrice);

  const { liquidityDepthStore } = useLiquidityDepthState({
    pairId,
    bucketSize: state.bucketSize,
    bucketRecords: state.bucketRecords,
    subscribe: true,
  });
  const liquidityDepth = liquidityDepthStore((s) => s.liquidityDepth);

  const { liveTradesStore } = useLiveTradesState({ pairId, subscribe: true });
  const liveTrades = liveTradesStore((s) => s.trades);

  const [historyTab, setHistoryTab] = useState("orders");

  const applySizeRatio = (ratio: number) => {
    const value = Decimal(state.maxSizeAmount).mul(ratio).toFixed(8);
    controllers.setValue("size", value.replace(/\.?0+$/, ""));
  };

  const size = inputs.size?.value || "0";
  const price = inputs.price?.value || "0";

  const canSubmit =
    isConnected &&
    !state.submission.isPending &&
    !state.isDexPaused &&
    Decimal(size || 0).gt(0) &&
    (state.operation === "market" || Decimal(price || 0).gt(0));

  return (
    <View className="flex-1 bg-surface-primary-rice px-4 pt-6 gap-3">
      <MobileTitle title={m["applets.trade.title"]()} />

      <ShadowContainer borderRadius={12}>
        <View className="rounded-xl bg-surface-secondary-rice p-3 gap-2">
          <View className="flex-row items-center justify-between">
            <Pressable onPress={() => setPairSheetVisible(true)}>
              <GlobalText className="h3-bold">{`${state.baseCoin.symbol}-${state.quoteCoin.symbol}`}</GlobalText>
            </Pressable>
            <GlobalText className="diatype-m-medium">
              {formatNumber(currentPrice, settings.formatNumberOptions)}
            </GlobalText>
          </View>
          <GlobalText className="diatype-sm-regular text-ink-tertiary-500">Spot</GlobalText>
        </View>
      </ShadowContainer>

      <Tabs
        fullWidth
        selectedTab={state.action}
        keys={["buy", "sell"]}
        onTabChange={(tab) => state.changeAction(tab as "buy" | "sell")}
      />

      <Tabs
        fullWidth
        selectedTab={state.operation}
        keys={["market", "limit"]}
        onTabChange={(tab) => state.setOperation(tab as "market" | "limit")}
      />

      {state.operation === "limit" ? (
        <Input
          label={m["dex.protrade.history.price"]()}
          value={inputs.price?.value || ""}
          keyboardType="decimal-pad"
          onChangeText={(v) => controllers.setValue("price", v || "0")}
          endContent={<GlobalText className="diatype-sm-medium">{state.quoteCoin.symbol}</GlobalText>}
        />
      ) : null}

      <Input
        label={m["dex.protrade.spot.orderSize"]()}
        value={inputs.size?.value || ""}
        keyboardType="decimal-pad"
        onChangeText={(v) => controllers.setValue("size", v || "0")}
        endContent={<GlobalText className="diatype-sm-medium">{state.sizeCoin.symbol}</GlobalText>}
      />

      <View className="flex-row gap-2">
        {[0.25, 0.5, 0.75, 1].map((ratio) => (
          <Button
            key={`trade-size-${ratio}`}
            size="xs"
            className="flex-1"
            variant="link"
            onPress={() => applySizeRatio(ratio)}
          >
            {`${ratio * 100}%`}
          </Button>
        ))}
      </View>

      <View className="flex-row justify-between">
        <GlobalText className="diatype-sm-regular text-ink-tertiary-500">
          {m["dex.protrade.spot.availableToTrade"]()}
        </GlobalText>
        <GlobalText className="diatype-sm-medium">
          {formatNumber(state.availableCoin.amount, settings.formatNumberOptions)} {state.availableCoin.symbol}
        </GlobalText>
      </View>

      <ChartPanel pairSymbols={normalized.pairSymbols} />

      <ShadowContainer borderRadius={12}>
        <View className="rounded-xl bg-surface-secondary-rice p-3 gap-2 min-h-44">
          <GlobalText className="diatype-sm-medium">{m["dex.protrade.openOrders"]()}</GlobalText>
          <View className="flex-row justify-between">
            <GlobalText className="diatype-xs-medium text-ink-tertiary-500">
              {m["dex.protrade.history.price"]()}
            </GlobalText>
            <GlobalText className="diatype-xs-medium text-ink-tertiary-500">
              {m["dex.protrade.history.total"]({ symbol: state.quoteCoin.symbol })}
            </GlobalText>
          </View>
          <ScrollView className="max-h-32">
            {liquidityDepth?.asks.records.slice(0, 8).map((ask, index) => (
              <View key={`ask-${ask.price}-${index}`} className="flex-row justify-between py-[2px]">
                <GlobalText className="diatype-xs-medium text-status-fail">{ask.price}</GlobalText>
                <GlobalText className="diatype-xs-medium">{ask.total}</GlobalText>
              </View>
            ))}
            {liquidityDepth?.bids.records.slice(0, 8).map((bid, index) => (
              <View key={`bid-${bid.price}-${index}`} className="flex-row justify-between py-[2px]">
                <GlobalText className="diatype-xs-medium text-status-success">{bid.price}</GlobalText>
                <GlobalText className="diatype-xs-medium">{bid.total}</GlobalText>
              </View>
            ))}
          </ScrollView>
        </View>
      </ShadowContainer>

      <ShadowContainer borderRadius={12}>
        <View className="rounded-xl bg-surface-secondary-rice p-3 gap-2 min-h-40">
          <GlobalText className="diatype-sm-medium">{m["dex.protrade.tradeHistory.title"]()}</GlobalText>
          <ScrollView className="max-h-32">
            {liveTrades.slice(0, 12).map((trade, index) => (
              <View key={`trade-${trade.createdAt}-${index}`} className="flex-row justify-between py-[2px]">
                <GlobalText className="diatype-xs-medium">
                  {formatNumber(
                    formatUnits(
                      trade.clearingPrice,
                      state.baseCoin.decimals - state.quoteCoin.decimals,
                    ),
                    settings.formatNumberOptions,
                  )}
                </GlobalText>
                <GlobalText className="diatype-xs-medium text-ink-tertiary-500">
                  {toTime(trade.createdAt)}
                </GlobalText>
              </View>
            ))}
          </ScrollView>
        </View>
      </ShadowContainer>

      <View className="mt-1">
        <Tabs
          fullWidth
          color="line-red"
          selectedTab={historyTab}
          keys={["orders", "trade-history"]}
          onTabChange={setHistoryTab}
        />
      </View>

      <View className="mb-2">
        <ShadowContainer borderRadius={12}>
          <View className="rounded-xl bg-surface-secondary-rice p-3 gap-2 min-h-44">
          {historyTab === "orders" ? (
            <>
              <View className="flex-row justify-between items-center">
                <GlobalText className="diatype-sm-medium">{m["dex.protrade.openOrders"]()}</GlobalText>
                {state.orders.data.length ? (
                  <Pressable
                    onPress={() =>
                      showModal(Modals.ProTradeCloseAll, {
                        ordersId: state.orders.data.map((order) => order.id),
                      })
                    }
                  >
                    <GlobalText className="diatype-xs-medium text-status-fail">
                      {m["modals.protradeCloseAllOrders.action"]()}
                    </GlobalText>
                  </Pressable>
                ) : null}
              </View>
              <FlatList
                data={state.orders.data}
                keyExtractor={(item) => `${item.id}`}
                renderItem={({ item }) => (
                  <View className="flex-row justify-between items-center py-2 border-b border-outline-secondary-gray">
                    <View className="gap-1">
                      <GlobalText className="diatype-xs-medium">{item.id}</GlobalText>
                      <GlobalText className="diatype-xs-regular text-ink-tertiary-500">
                        {item.direction}
                      </GlobalText>
                    </View>
                    <Pressable onPress={() => showModal(Modals.ProTradeCloseOrder, { orderId: item.id })}>
                      <GlobalText className="diatype-xs-medium text-status-fail">
                        {m["modals.proTradeCloseOrder.action"]()}
                      </GlobalText>
                    </Pressable>
                  </View>
                )}
                ListEmptyComponent={
                  <GlobalText className="diatype-sm-regular text-ink-tertiary-500">No data</GlobalText>
                }
              />
            </>
          ) : (
            <FlatList
              data={state.history.data?.nodes || []}
              keyExtractor={(_, index) => `history-${index}`}
              renderItem={({ item }) => (
                <View className="flex-row justify-between items-center py-2 border-b border-outline-secondary-gray">
                  <GlobalText className="diatype-xs-medium">
                    {item?.direction || "-"}
                  </GlobalText>
                  <GlobalText className="diatype-xs-regular text-ink-tertiary-500">
                    {item?.createdAt ? toTime(item.createdAt) : "-"}
                  </GlobalText>
                </View>
              )}
              ListEmptyComponent={
                <GlobalText className="diatype-sm-regular text-ink-tertiary-500">No data</GlobalText>
              }
            />
          )}
          </View>
        </ShadowContainer>
      </View>

      {isConnected ? (
        <Button isLoading={state.submission.isPending} isDisabled={!canSubmit} onPress={() => state.submission.mutate()}>
          {m["dex.protrade.spot.triggerAction"]({ action: state.action })} {state.baseCoin.symbol}
        </Button>
      ) : (
        <Button onPress={() => showModal(Modals.Authenticate, { action: "signin" })}>
          {m["dex.protrade.spot.enableTrading"]()}
        </Button>
      )}

      <PairSelectSheet
        isVisible={pairSheetVisible}
        onClose={() => setPairSheetVisible(false)}
        onSelect={(nextPairId) => {
          setPairSheetVisible(false);
          state.onChangePairId(nextPairId);
        }}
      />
    </View>
  );
};

type PairSelectSheetProps = {
  isVisible: boolean;
  onClose: () => void;
  onSelect: (pairId: PairId) => void;
};

const PairSelectSheet: React.FC<PairSelectSheetProps> = ({ isVisible, onClose, onSelect }) => {
  const { data: appConfig } = useAppConfig();
  const { coins } = useConfig();
  const [searchText, setSearchText] = useState("");

  const pairs = useMemo(() => {
    const values = Object.values(appConfig?.pairs || {});
    const filtered = values.filter((pair) => !pair.baseDenom.includes("dango"));
    const unique = new Map<string, PairId>();

    filtered.forEach((pair) => {
      const baseSymbol = coins.byDenom[pair.baseDenom]?.symbol;
      const quoteSymbol = coins.byDenom[pair.quoteDenom]?.symbol;
      if (!baseSymbol || !quoteSymbol) return;
      const key = `${baseSymbol}-${quoteSymbol}`;
      if (!unique.has(key)) {
        unique.set(key, { baseDenom: pair.baseDenom, quoteDenom: pair.quoteDenom });
      }
    });

    return Array.from(unique.entries())
      .filter(([symbol]) => symbol.toLowerCase().includes(searchText.toLowerCase()))
      .map(([symbol, pairId]) => ({ symbol, pairId }));
  }, [appConfig?.pairs, coins.byDenom, searchText]);

  return (
    <Modal visible={isVisible} transparent animationType="fade" onRequestClose={onClose}>
      <View className="flex-1 justify-end">
        <Pressable className="absolute inset-0 bg-primitives-gray-light-900/50" onPress={onClose} />
        <View className="bg-surface-primary-rice rounded-t-2xl px-4 pt-4 pb-8 gap-3 max-h-[70%]">
          <GlobalText className="h4-bold">{m["dex.tokens"]()}</GlobalText>
          <TextInput
            value={searchText}
            onChangeText={setSearchText}
            placeholder={m["dex.searchFor"]()}
            className="w-full h-11 px-3 rounded-lg bg-surface-secondary-rice text-ink-primary-900"
          />
          <FlatList
            data={pairs}
            keyExtractor={(item) => item.symbol}
            renderItem={({ item }) => (
              <Pressable
                onPress={() => onSelect(item.pairId)}
                className="py-3 border-b border-outline-secondary-gray"
              >
                <GlobalText className="diatype-m-medium">{item.symbol}</GlobalText>
              </Pressable>
            )}
            ListEmptyComponent={
              <GlobalText className="diatype-sm-regular text-ink-tertiary-500">No data</GlobalText>
            }
          />
        </View>
      </View>
    </Modal>
  );
};

const ChartPanel: React.FC<{ pairSymbols: string }> = ({ pairSymbols }) => {
  const { theme } = useTheme();

  const uri = `${PORTAL_WEB_ORIGIN}/trade/${pairSymbols}?embed=chart&theme=${theme}`;

  return (
    <ShadowContainer borderRadius={12}>
      <View className="overflow-hidden rounded-xl bg-surface-secondary-rice h-64">
        <WebView source={{ uri }} style={{ flex: 1 }} />
      </View>
    </ShadowContainer>
  );
};
const toTime = (value: string | number) => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleTimeString();
};
