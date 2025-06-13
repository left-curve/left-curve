import { useInputs, useWatchEffect } from "@left-curve/applets-kit";
import { useAccount, useBalances, useConfig, usePrices, useSigningClient } from "@left-curve/store";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { createLazyFileRoute, useNavigate, useSearch } from "@tanstack/react-router";
import { useState } from "react";
import { useApp } from "~/hooks/useApp";

import {
  AccountSearchInput,
  Button,
  CoinSelector,
  Input,
  QRCode,
  ResizerContainer,
  Tabs,
  TextCopy,
  TruncateText,
} from "@left-curve/applets-kit";
import type { Address } from "@left-curve/dango/types";
import { MobileTitle } from "~/components/foundation/MobileTitle";
import { Modals } from "~/components/modals/RootModal";

import { toast } from "~/components/foundation/Toast";
import { m } from "~/paraglide/messages";

import { isValidAddress } from "@left-curve/dango";
import {
  capitalize,
  formatNumber,
  formatUnits,
  parseUnits,
  withResolvers,
} from "@left-curve/dango/utils";

export const Route = createLazyFileRoute("/(app)/_app/transfer")({
  component: TransferApplet,
});

function TransferApplet() {
  const { action } = useSearch({ strict: false });
  const navigate = useNavigate({ from: "/transfer" });
  const { settings, showModal, subscriptions } = useApp();
  const { formatNumberOptions } = settings;

  const queryClient = useQueryClient();
  const setAction = (v: string) => navigate({ search: { action: v }, replace: false });
  const [selectedDenom, setSelectedDenom] = useState("bridge/usdc");
  const { register, setValue, reset, handleSubmit, inputs } = useInputs({
    strategy: "onSubmit",
  });

  const { account, isConnected } = useAccount();
  const { coins } = useConfig();
  const { data: signingClient } = useSigningClient();

  const { data: balances = {}, refetch: refreshBalances } = useBalances({
    address: account?.address,
  });

  useWatchEffect(isConnected, (v) => !v && setAction("send"));

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

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
      subscriptions.emit("submitTx", { isSubmitting: true });
      try {
        const parsedAmount = parseUnits(amount, selectedCoin.decimals).toString();

        const { promise, resolve: confirmSend, reject: rejectSend } = withResolvers();

        showModal(Modals.ConfirmSend, {
          amount: parsedAmount,
          denom: selectedDenom,
          to: address as Address,
          confirmSend,
          rejectSend,
        });

        const response = await promise
          .then(() => true)
          .catch(() => {
            subscriptions.emit("submitTx", {
              isSubmitting: false,
              txResult: { hasSucceeded: false, message: m["transfer.error.description"]() },
            });
            return false;
          });

        if (!response) return undefined;

        await signingClient.transfer({
          transfer: {
            [address]: {
              [selectedCoin.denom]: parsedAmount,
            },
          },
          sender: account!.address as Address,
        });

        reset();
        toast.success({ title: m["sendAndReceive.sendSuccessfully"]() });
        subscriptions.emit("submitTx", {
          isSubmitting: false,
          txResult: { hasSucceeded: true, message: m["sendAndReceive.sendSuccessfully"]() },
        });
        refreshBalances();
        queryClient.invalidateQueries({ queryKey: ["quests", account] });
      } catch (e) {
        console.error(e);
        subscriptions.emit("submitTx", {
          isSubmitting: false,
          txResult: { hasSucceeded: false, message: m["transfer.error.description"]() },
        });
        toast.error(
          {
            title: m["transfer.error.title"](),
            description: m["transfer.error.description"](),
          },
          {
            duration: Number.POSITIVE_INFINITY,
          },
        );
      }
    },
  });

  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
      <MobileTitle title={m["sendAndReceive.title"]()} />

      <div className="w-full flex flex-col gap-4  md:pt-28 items-center justify-start ">
        <ResizerContainer
          layoutId="send-and-receive"
          className="max-w-[400px] flex flex-col gap-8 rounded-xl w-full"
        >
          <Tabs
            layoutId="tabs-send-and-receive"
            selectedTab={isConnected ? action : "send"}
            keys={isConnected ? ["send", "receive"] : ["send"]}
            fullWidth
            onTabChange={setAction}
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
                      if (!v) return m["errors.validations.amountIsRequired"]();
                      if (Number(v) <= 0) return m["errors.validations.amountIsZero"]();
                      if (Number(v) > Number(humanAmount))
                        return m["errors.validations.insufficientFunds"]();
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
                      coins={Object.values(coins)}
                      value={selectedDenom}
                      isDisabled={isPending}
                      onChange={(k) => [setSelectedDenom(k)]}
                    />
                  }
                  insideBottomComponent={
                    <div className="w-full flex justify-between pl-4 h-[22px]">
                      <div className="flex gap-1 items-center justify-center diatype-sm-regular text-tertiary-500">
                        <span>
                          {formatNumber(humanAmount, {
                            ...formatNumberOptions,
                            notation: "compact",
                            maxFractionDigits: selectedCoin.decimals / 3,
                          })}
                        </span>
                        <Button
                          type="button"
                          isDisabled={isPending}
                          variant="secondary"
                          size="xs"
                          className="bg-primary-red text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
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
                    validate: (v) => isValidAddress(v) || m["errors.validations.invalidAddress"](),
                    mask: (v) => v.toLowerCase().replace(/[^a-z0-9_]/g, ""),
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
                isDisabled={!isConnected || !!inputs.amount?.error}
              >
                {m["common.send"]()}
              </Button>
            </form>
          ) : (
            <div className="flex flex-col w-full gap-6 items-center justify-center text-center pb-10 bg-bg-secondary-rice rounded-xl shadow-account-card p-4">
              <div className="flex flex-col gap-1 items-center">
                <p className="exposure-h3-italic">{`${capitalize((account?.type as string) || "")} Account #${account?.index}`}</p>
                <div className="flex gap-1">
                  <TruncateText
                    className="diatype-sm-medium text-tertiary-500"
                    text={account?.address}
                  />
                  <TextCopy
                    copyText={account?.address}
                    className="w-4 h-4 cursor-pointer text-tertiary-500"
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
