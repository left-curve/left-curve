import { forwardRef, useImperativeHandle } from "react";
import { View } from "react-native";

import { useConfig, usePrices } from "@left-curve/store";
import { formatUnits } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Button, CoinIcon, GlobalText } from "~/components/foundation";

import type { Coin } from "@left-curve/dango/types";

export type SheetRef = {
  triggerOnClose: () => void;
};

type ConfirmSwapSheetProps = {
  input: {
    coin: Coin;
    amount: string;
  };
  output: {
    coin: Coin;
    amount: string;
  };
  fee: string;
  confirmSwap: () => void;
  rejectSwap: () => void;
  closeSheet: () => void;
};

export const ConfirmSwapSheet = forwardRef<SheetRef, ConfirmSwapSheetProps>(
  ({ input, output, fee, confirmSwap, rejectSwap, closeSheet }, ref) => {
    const { coins } = useConfig();
    const { getPrice } = usePrices();

    const inputCoin = coins.byDenom[input.coin.denom];
    const outputCoin = coins.byDenom[output.coin.denom];

    const inputAmount = formatUnits(input.amount, inputCoin.decimals);
    const outputAmount = formatUnits(output.amount, outputCoin.decimals);

    useImperativeHandle(ref, () => ({
      triggerOnClose: rejectSwap,
    }));

    return (
      <View className="flex flex-col gap-4">
        <View className="flex flex-col gap-2">
          <GlobalText className="exposure-sm-italic text-ink-tertiary-500">
            {m["dex.swapping"]()}
          </GlobalText>
          <View className="flex-row items-center justify-between">
            <GlobalText className="h3-bold">{`${inputAmount} ${inputCoin.symbol}`}</GlobalText>
            <CoinIcon symbol={inputCoin.symbol} size={28} />
          </View>
          <GlobalText className="diatype-sm-regular text-ink-tertiary-500">
            {getPrice(inputAmount, inputCoin.denom, { format: true })}
          </GlobalText>
        </View>

        <View className="flex flex-col gap-2">
          <View className="flex-row items-center justify-between">
            <GlobalText className="h3-bold">{`${outputAmount} ${outputCoin.symbol}`}</GlobalText>
            <CoinIcon symbol={outputCoin.symbol} size={28} />
          </View>
          <GlobalText className="diatype-sm-regular text-ink-tertiary-500">
            {getPrice(outputAmount, outputCoin.denom, { format: true })}
          </GlobalText>
        </View>

        <View className="flex-row items-center justify-between">
          <GlobalText className="diatype-sm-regular text-ink-tertiary-500">{m["dex.fee"]()}</GlobalText>
          <GlobalText className="diatype-sm-medium">{fee}</GlobalText>
        </View>

        <Button
          onPress={() => {
            confirmSwap();
            closeSheet();
          }}
        >
          {m["common.confirm"]()}
        </Button>
      </View>
    );
  },
);

ConfirmSwapSheet.displayName = "ConfirmSwapSheet";
