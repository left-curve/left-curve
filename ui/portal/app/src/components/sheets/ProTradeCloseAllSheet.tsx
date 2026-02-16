import { useAccount, useSigningClient, useSubmitTx } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useQueryClient } from "@tanstack/react-query";
import { forwardRef } from "react";
import { View } from "react-native";

import { Button, GlobalText } from "~/components/foundation";

import type { OrderId } from "@left-curve/dango/types";

type SheetRef = {
  triggerOnClose: () => void;
};

type ProTradeCloseAllSheetProps = {
  ordersId: OrderId[];
  closeSheet: () => void;
};

export const ProTradeCloseAllSheet = forwardRef<SheetRef, ProTradeCloseAllSheetProps>(
  ({ ordersId, closeSheet }, _ref) => {
    const { account } = useAccount();
    const { data: signingClient } = useSigningClient();
    const queryClient = useQueryClient();

    const { mutateAsync: cancelAllOrders, isPending } = useSubmitTx({
      submission: {
        success: m["dex.protrade.allOrdersCancelled"](),
        error: m["errors.failureRequest"](),
      },
      mutation: {
        mutationFn: async () => {
          if (!signingClient || !account) throw new Error("No signing client available");
          await signingClient.batchUpdateOrders({
            cancels: { some: ordersId },
            sender: account.address,
          });
        },
        onSuccess: async () => {
          await queryClient.invalidateQueries({ queryKey: ["ordersByUser", account?.address] });
          closeSheet();
        },
      },
    });

    return (
      <View className="gap-4">
        <GlobalText className="diatype-sm-regular text-ink-tertiary-500">
          {m["modals.protradeCloseAllOrders.description"]()}
        </GlobalText>
        <Button isLoading={isPending} onPress={() => cancelAllOrders()}>
          {m["modals.protradeCloseAllOrders.action"]()}
        </Button>
      </View>
    );
  },
);

ProTradeCloseAllSheet.displayName = "ProTradeCloseAllSheet";
