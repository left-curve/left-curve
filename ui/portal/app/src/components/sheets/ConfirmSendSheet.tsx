import { forwardRef, useImperativeHandle } from "react";
import { useQuery } from "@tanstack/react-query";
import { View } from "react-native";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { formatUnits } from "@left-curve/dango/utils";
import { useConfig, usePrices, usePublicClient } from "@left-curve/store";

import { Button, CoinIcon, GlobalText, TruncateText } from "~/components/foundation";

import type { Address } from "@left-curve/dango/types";
import type { SheetRef } from "./ConfirmSwapSheet";

type ConfirmSendSheetProps = {
  amount: string;
  denom: string;
  to: Address;
  confirmSend: () => void;
  rejectSend: () => void;
  closeSheet: () => void;
};

export const ConfirmSendSheet = forwardRef<SheetRef, ConfirmSendSheetProps>(
  ({ amount, denom, to, confirmSend, rejectSend, closeSheet }, ref) => {
    const { getCoinInfo } = useConfig();
    const { getPrice } = usePrices();
    const client = usePublicClient();

    const coin = getCoinInfo(denom);
    const humanAmount = formatUnits(amount, coin.decimals);

    const { data: username } = useQuery({
      queryKey: ["username", to],
      queryFn: async () => {
        const response = await client.getAccountInfo({ address: to });
        if (!response) return "Unknown account";
        const { index, params } = response;
        const [type, config] = Object.entries(params)[0];
        return `${type === "multi" ? "Multisig" : String(config.owner)} #${index}`;
      },
    });

    useImperativeHandle(ref, () => ({
      triggerOnClose: rejectSend,
    }));

    return (
      <View className="flex flex-col gap-4">
        <View className="flex flex-col gap-2">
          <GlobalText className="exposure-sm-italic text-ink-tertiary-500">
            {m["modals.confirmSend.sending"]()}
          </GlobalText>
          <View className="flex-row items-center justify-between">
            <GlobalText className="h3-bold">{`${humanAmount} ${coin.symbol}`}</GlobalText>
            <CoinIcon symbol={coin.symbol} size={28} />
          </View>
          <GlobalText className="diatype-sm-regular text-ink-tertiary-500">
            {getPrice(humanAmount, denom, { format: true })}
          </GlobalText>
        </View>

        <View className="flex flex-col gap-1">
          <GlobalText className="exposure-sm-italic text-ink-tertiary-500">{m["common.to"]()}</GlobalText>
          <GlobalText className="h3-bold">{username || "..."}</GlobalText>
          <TruncateText text={to} className="diatype-sm-regular text-ink-tertiary-500" />
        </View>

        <Button
          onPress={() => {
            confirmSend();
            closeSheet();
          }}
        >
          {m["modals.confirmSend.confirmButton"]()}
        </Button>
      </View>
    );
  },
);

ConfirmSendSheet.displayName = "ConfirmSendSheet";
