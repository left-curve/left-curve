import {
  capitalize,
  formatNumber,
  formatUnits,
  parseUnits,
  wait,
  withResolvers,
} from "@left-curve/dango/utils";
import {
  useAccount,
  useBalances,
  useChainId,
  useConfig,
  usePrices,
  useSigningClient,
} from "@left-curve/store";
import { createFileRoute, useNavigate, useSearch } from "@tanstack/react-router";
import { useState } from "react";
import { useApp } from "~/hooks/useApp";

import {
  AccountSearchInput,
  Button,
  CoinSelector,
  IconCopy,
  Input,
  QRCode,
  ResizerContainer,
  Tabs,
  TruncateText,
  useInputs,
} from "@left-curve/applets-kit";
import { isValidAddress } from "@left-curve/dango";
import type { Address } from "@left-curve/dango/types";
import { useMutation } from "@tanstack/react-query";

import { z } from "zod";
import { Modals } from "~/components/foundation/Modal";
import { toast } from "~/components/foundation/Toast";
import { m } from "~/paraglide/messages";

const searchParamsSchema = z.object({
  action: z.enum(["send", "receive"]).catch("send"),
});

export const Route = createFileRoute("/(app)/_app/send-and-receive")({
  component: SendAndReceiveComponent,
  validateSearch: searchParamsSchema,
});

function SendAndReceiveComponent() {
  const { action } = useSearch({ strict: false });
  const navigate = useNavigate({ from: "/send-and-receive" });
  const { formatNumberOptions, showModal } = useApp();

  const [selectedDenom, setSelectedDenom] = useState("uusdc");
  const { register, setValue, reset, handleSubmit, inputs } = useInputs({ strategy: "onSubmit" });

  const { account, isConnected } = useAccount();
  const chainId = useChainId();
  const config = useConfig();
  const { data: signingClient } = useSigningClient();

  const { data: balances = {}, refetch: refreshBalances } = useBalances({
    address: account?.address,
  });

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  const coins = config.coins[chainId];
  const selectedCoin = coins[selectedDenom];

  const humanAmount = formatUnits(balances[selectedDenom] || 0, selectedCoin.decimals);

  const price = getPrice(inputs.amount?.value || "0", selectedDenom, {
    format: true,
    formatOptions: { ...formatNumberOptions, currency: "USD" },
  });

  const { mutateAsync: onSubmit, isPending } = useMutation<
    void,
    Error,
    { amount: string; address: string }
  >({
    mutationFn: async ({ address, amount }) => {
      if (!signingClient) throw new Error("error: no signing client");
      const parsedAmount = parseUnits(amount, selectedCoin.decimals).toString();

      const { promise, resolve: confirmSend, reject: rejectSend } = withResolvers();

      showModal(Modals.ConfirmSend, {
        amount: parsedAmount,
        denom: selectedDenom,
        to: address as Address,
        confirmSend,
        rejectSend,
      });

      const response = await promise.then(() => true).catch(() => false);

      if (!response) return undefined;

      await signingClient.transfer({
        transfer: {
          [address]: {
            [selectedCoin.denom]: parsedAmount,
          },
        },
        sender: account!.address as Address,
      });

      await wait(1000);
      reset();
      toast.success({ title: m["sendAndReceive.sendSuccessfully"]() });
      refreshBalances();
    },

    onError: (e) => {
      toast.error({
        title: m["common.error"](),
        description: e.message,
      });
    },
  });

  return (
    <div className="flex w-full justify-center items-center">
      <div className="w-full md:max-w-[50rem] flex flex-col gap-4 p-4 md:pt-28 items-center justify-start ">
        <ResizerContainer
          layoutId="send-and-receive"
          className="p-6 shadow-card-shadow max-w-[400px] bg-rice-25 flex flex-col gap-8 rounded-3xl w-full"
        >
          <Tabs
            layoutId="tabs-send-and-receive"
            selectedTab={isConnected ? action : "send"}
            keys={isConnected ? ["send", "receive"] : ["send"]}
            fullWidth
            onTabChange={(v) => navigate({ search: { action: v }, replace: false })}
          />

          {action === "send" ? (
            <form onSubmit={handleSubmit(onSubmit)}>
              <div className="flex flex-col w-full gap-4">
                <Input
                  label="You're sending"
                  placeholder="0"
                  classNames={{
                    base: "z-20",
                    inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px]",
                    inputParent: "h-[34px] h3-bold",
                    input: "!h3-bold",
                  }}
                  isDisabled={isPending}
                  {...register("amount", {
                    strategy: "onChange",
                    validate: (v) => {
                      if (!v) return m["validations.errors.amountIsRequired"]();
                      if (Number(v) <= 0) return m["validations.errors.amountIsZero"]();
                      if (Number(v) > Number(humanAmount))
                        return m["validations.errors.insufficientFunds"]();
                      return true;
                    },
                    mask: (v, prev) => {
                      const regex = /^\d+(\.\d{0,18})?$/;
                      if (v === "" || regex.test(v)) return v;
                      return prev;
                    },
                  })}
                  startText="right"
                  startContent={
                    <CoinSelector
                      label="coins"
                      classNames={{ trigger: "p-0" }}
                      coins={Object.values(coins)}
                      selectedKey={selectedDenom}
                      isDisabled={isPending}
                      onSelectionChange={(k) => [setSelectedDenom(k.toString())]}
                    />
                  }
                  insideBottomComponent={
                    <div className="w-full flex justify-between pl-4 h-[22px]">
                      <div className="flex gap-1 items-center justify-center diatype-sm-regular text-gray-500">
                        <span>
                          {formatNumber(humanAmount, {
                            ...formatNumberOptions,
                            notation: "compact",
                            maxFractionDigits: selectedCoin.decimals / 3,
                          })}
                        </span>
                        <Button
                          isDisabled={isPending}
                          variant="secondary"
                          size="xs"
                          className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                          onClick={() => setValue("amount", humanAmount)}
                        >
                          {m["common.max"]()}
                        </Button>
                      </div>
                      <p>{price}</p>
                    </div>
                  }
                />
                <AccountSearchInput
                  {...register("address", {
                    validate: (v) => isValidAddress(v) || m["validations.errors.invalidAddress"](),
                    mask: (v) => v.toLowerCase(),
                  })}
                  label="To"
                  placeholder="Wallet address or name"
                  isDisabled={isPending}
                />
              </div>

              <Button
                type="submit"
                fullWidth
                className="mt-5"
                isLoading={isPending}
                isDisabled={!isConnected || !!inputs.amount?.error || !!inputs.address?.error}
              >
                {m["common.send"]()}
              </Button>
            </form>
          ) : (
            <div className="flex flex-col w-full gap-14 items-center justify-center text-center pb-10">
              <div className="flex flex-col gap-1 items-center">
                <p className="exposure-h3-italic">{`${capitalize(account?.type as string)} Account #${account?.index}`}</p>
                <div className="flex gap-1">
                  <TruncateText
                    className="diatype-sm-medium text-gray-500"
                    text={account?.address}
                  />
                  <IconCopy
                    copyText={account?.address}
                    className="w-4 h-4 cursor-pointer text-gray-500"
                  />
                </div>
              </div>
              <QRCode data={account?.address as string} />
            </div>
          )}
        </ResizerContainer>
      </div>
    </div>
  );
}
