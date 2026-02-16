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

type ProTradeCloseOrderSheetProps = {
  orderId: OrderId;
  closeSheet: () => void;
};

export const ProTradeCloseOrderSheet = forwardRef<SheetRef, ProTradeCloseOrderSheetProps>(
  ({ orderId, closeSheet }, _ref) => {
    const { account } = useAccount();
    const { data: signingClient } = useSigningClient();
    const queryClient = useQueryClient();

    const { mutateAsync: cancelOrder, isPending } = useSubmitTx({
      submission: {
        success: m["dex.protrade.allOrdersCancelled"](),
        error: m["errors.failureRequest"](),
      },
      mutation: {
        mutationFn: async () => {
          if (!signingClient || !account) throw new Error("No signing client available");
          await signingClient.batchUpdateOrders({
            cancels: { some: [orderId] },
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
          {m["modals.proTradeCloseOrder.description"]()}
        </GlobalText>
        <Button isLoading={isPending} onPress={() => cancelOrder()}>
          {m["modals.proTradeCloseOrder.action"]()}
        </Button>
      </View>
    );
  },
);

ProTradeCloseOrderSheet.displayName = "ProTradeCloseOrderSheet";
