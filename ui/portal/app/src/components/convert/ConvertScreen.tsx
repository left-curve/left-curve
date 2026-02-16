import { useApp, Modals } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useConfig, useConvertState } from "@left-curve/store";
import { useLocalSearchParams, useRouter } from "expo-router";
import { useEffect, useMemo, useState } from "react";
import { TextInput, View } from "react-native";
import { Picker } from "@react-native-picker/picker";

import { formatNumber, formatUnits, withResolvers } from "@left-curve/dango/utils";

import { normalizeConvertParams } from "~/features/convert/params";
import { Button, CoinIcon, GlobalText, MobileTitle, ShadowContainer } from "~/components/foundation";

const BASE_SYMBOL = "USDC";

export const ConvertScreen: React.FC = () => {
  const router = useRouter();
  const params = useLocalSearchParams<{ from?: string | string[]; to?: string | string[] }>();

  const { isConnected } = useAccount();
  const { showModal, settings } = useApp();
  const { coins } = useConfig();

  const pair = useMemo(() => normalizeConvertParams(params, coins), [params, coins]);

  useEffect(() => {
    if (!pair.changed) return;
    router.setParams({ from: pair.from, to: pair.to });
  }, [pair.changed, pair.from, pair.to, router]);

  const [inputs, setInputs] = useState<Record<string, { value: string }>>({
    from: { value: "0" },
    to: { value: "0" },
  });

  const [activeInput, setActiveInput] = useState<"from" | "to">("from");

  const controllers = useMemo(
    () => ({
      inputs,
      reset: () => setInputs({ from: { value: "0" }, to: { value: "0" } }),
      setValue: (name: string, value: string) =>
        setInputs((prev) => ({
          ...prev,
          [name]: { value },
        })),
    }),
    [inputs],
  );

  const convertState = useConvertState({
    pair: { from: pair.from, to: pair.to },
    controllers,
    onChangePair: (nextPair) => {
      router.setParams({ from: nextPair.from, to: nextPair.to });
    },
    submission: {
      confirm: async () => {
        if (!convertState.simulation.data) return;

        const { promise, resolve: confirmSwap, reject: rejectSwap } = withResolvers<void>();

        showModal(Modals.ConfirmSwap, {
          input: {
            coin: convertState.coins.byDenom[convertState.simulation.data.input.denom],
            amount: convertState.simulation.data.input.amount,
          },
          output: {
            coin: convertState.coins.byDenom[convertState.simulation.data.output.denom],
            amount: convertState.simulation.data.output.amount,
          },
          fee: formatNumber(convertState.fee, {
            ...settings.formatNumberOptions,
            currency: "usd",
          }),
          confirmSwap,
          rejectSwap,
        });

        await promise;
      },
      onError: () => null,
    },
    simulation: {
      onError: () => null,
    },
  });

  useEffect(() => {
    const timeout = setTimeout(() => {
      void convertState.simulation.mutateAsync(activeInput).catch(() => null);
    }, 300);

    return () => clearTimeout(timeout);
  }, [
    activeInput,
    convertState.simulation,
    inputs.from?.value,
    inputs.to?.value,
    pair.from,
    pair.to,
  ]);

  const fromCoin = coins.bySymbol[pair.from];
  const toCoin = coins.bySymbol[pair.to];

  if (!fromCoin || !toCoin) return null;

  const symbols = Object.keys(coins.bySymbol);

  const handleSelectFrom = (symbol: string) => {
    if (symbol === BASE_SYMBOL) {
      const nextTo = pair.to === BASE_SYMBOL ? "ETH" : pair.to;
      router.setParams({ from: BASE_SYMBOL, to: nextTo });
      return;
    }
    router.setParams({ from: symbol, to: BASE_SYMBOL });
  };

  const handleSelectTo = (symbol: string) => {
    if (symbol === BASE_SYMBOL) {
      const nextFrom = pair.from === BASE_SYMBOL ? "ETH" : pair.from;
      router.setParams({ from: nextFrom, to: BASE_SYMBOL });
      return;
    }
    router.setParams({ from: BASE_SYMBOL, to: symbol });
  };

  const rate = (() => {
    const simulation = convertState.simulation.data;
    if (!simulation) return "-";

    const inputAmount = Number(formatUnits(simulation.input.amount, fromCoin.decimals));
    const outputAmount = Number(formatUnits(simulation.output.amount, toCoin.decimals));
    if (!inputAmount || Number.isNaN(inputAmount)) return "-";

    return outputAmount / inputAmount;
  })();

  return (
    <View className="flex-1 bg-surface-primary-rice px-4 pt-6 gap-4">
      <MobileTitle title={m["dex.convert.title"]()} />

      <ShadowContainer borderRadius={16}>
        <View className="rounded-xl bg-surface-tertiary-rice p-4 gap-2">
          <View className="flex-row items-center justify-between">
            <View className="flex-row items-center gap-2">
              <CoinIcon symbol={fromCoin.symbol} size={22} />
              <GlobalText className="h4-bold">{fromCoin.symbol}</GlobalText>
            </View>
          </View>
        </View>
      </ShadowContainer>

      <View className="gap-4">
        <View className="gap-2">
          <GlobalText className="exposure-sm-italic text-ink-tertiary-500">
            {m["dex.convert.youSwap"]()}
          </GlobalText>
          <ShadowContainer borderRadius={12}>
            <View className="rounded-lg bg-surface-secondary-rice p-2 gap-2">
              <TextInput
                value={inputs.from?.value || "0"}
                keyboardType="decimal-pad"
                onFocus={() => setActiveInput("from")}
                onChangeText={(value) => controllers.setValue("from", value || "0")}
                className="text-ink-secondary-700 diatype-lg-medium"
              />
              <Picker selectedValue={pair.from} onValueChange={handleSelectFrom}>
                {symbols.map((symbol) => (
                  <Picker.Item label={symbol} value={symbol} key={`convert-from-${symbol}`} />
                ))}
              </Picker>
            </View>
          </ShadowContainer>
        </View>

        <View className="gap-2">
          <GlobalText className="exposure-sm-italic text-ink-tertiary-500">
            {m["dex.convert.youGet"]()}
          </GlobalText>
          <ShadowContainer borderRadius={12}>
            <View className="rounded-lg bg-surface-secondary-rice p-2 gap-2">
              <TextInput
                value={inputs.to?.value || "0"}
                keyboardType="decimal-pad"
                onFocus={() => setActiveInput("to")}
                onChangeText={(value) => controllers.setValue("to", value || "0")}
                className="text-ink-secondary-700 diatype-lg-medium"
              />
              <Picker selectedValue={pair.to} onValueChange={handleSelectTo}>
                {symbols.map((symbol) => (
                  <Picker.Item label={symbol} value={symbol} key={`convert-to-${symbol}`} />
                ))}
              </Picker>
            </View>
          </ShadowContainer>
        </View>
      </View>

      <View className="gap-1">
        <View className="flex-row justify-between">
          <GlobalText className="diatype-sm-regular text-ink-tertiary-500">{m["dex.fee"]()}</GlobalText>
          <GlobalText className="diatype-sm-medium">
            {formatNumber(convertState.fee, { ...settings.formatNumberOptions, currency: "usd" })}
          </GlobalText>
        </View>
        <View className="flex-row justify-between">
          <GlobalText className="diatype-sm-regular text-ink-tertiary-500">
            {m["dex.convert.rate"]()}
          </GlobalText>
          <GlobalText className="diatype-sm-medium">{`1 ${fromCoin.symbol} ≈ ${rate} ${toCoin.symbol}`}</GlobalText>
        </View>
      </View>

      {isConnected ? (
        <Button
          isLoading={convertState.submission.isPending}
          isDisabled={
            Number(convertState.simulation.data?.output.amount || 0) <= 0 ||
            convertState.simulation.isPending
          }
          onPress={() => convertState.submission.mutate()}
        >
          {m["dex.convert.swap"]()}
        </Button>
      ) : (
        <Button onPress={() => showModal(Modals.Authenticate, { action: "signin" })}>
          {m["common.signin"]()}
        </Button>
      )}
    </View>
  );
};
