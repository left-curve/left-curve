import {
  AssetInputWithRange,
  Modals,
  useApp,
  useInputs,
  useWatchEffect,
} from "@left-curve/applets-kit";
import {
  useAccount,
  useBalances,
  useConfig,
  usePublicClient,
  useSigningClient,
  useSubmitTx,
} from "@left-curve/store";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import {
  AccountSearchInput,
  Button,
  CoinSelector,
  QRCode,
  ResizerContainer,
  Tabs,
  TextCopy,
  TruncateText,
} from "@left-curve/applets-kit";
import type { Address } from "@left-curve/dango/types";
import { MobileTitle } from "~/components/foundation/MobileTitle";
import { WarningTransferAccounts } from "~/components/transfer/WarningTransferAccounts";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { isValidAddress } from "@left-curve/dango";
import { capitalize, parseUnits, wait, withResolvers } from "@left-curve/dango/utils";

export const Route = createLazyFileRoute("/(app)/_app/transfer")({
  component: TransferApplet,
});

function TransferApplet() {
  const { action } = Route.useSearch();
  const navigate = useNavigate({ from: "/transfer" });
  const { showModal } = useApp();

  const queryClient = useQueryClient();
  const setAction = (v: string) => navigate({ search: { action: v }, replace: true });
  const [selectedDenom, setSelectedDenom] = useState("bridge/usdc");
  const controllers = useInputs({
    strategy: "onSubmit",
  });

  const { register, reset, handleSubmit, inputs } = controllers;

  const { account, isConnected } = useAccount();
  const { coins } = useConfig();
  const { data: signingClient } = useSigningClient();
  const publicClient = usePublicClient();

  const { refetch: refreshBalances, data: balances = {} } = useBalances({
    address: account?.address,
  });

  useWatchEffect(isConnected, (v) => !v && setAction("send"));

  const isValid20HexAddress = isValidAddress(inputs.address?.value || "");

  const { data: doesUserExist = false, isFetching } = useQuery({
    enabled: !!inputs.address?.value?.length,
    queryKey: ["transfer", inputs.address?.value],
    queryFn: async ({ signal }) => {
      await wait(450);
      if (signal.aborted || !isValid20HexAddress) return null;

      const account = await publicClient.getAccountInfo({
        address: inputs.address?.value as Address,
      });

      if (!account) return false;
      return true;
    },
  });

  const showAdressWarning =
    action === "send" && inputs.address?.value && isValid20HexAddress && !doesUserExist;

  const selectedCoin = coins.byDenom[selectedDenom];

  const { mutateAsync: onSubmit, isPending } = useSubmitTx<
    void,
    Error,
    { amount: string; address: string }
  >({
    submission: {
      success: m["sendAndReceive.sendSuccessfully"](),
      error: m["transfer.error.description"](),
    },
    mutation: {
      mutationFn: async ({ address, amount }, { abort }) => {
        if (!signingClient) throw new Error("error: no signing client");

        const parsedAmount = parseUnits(amount, selectedCoin.decimals);

        const { promise, resolve: confirmSend, reject: rejectSend } = withResolvers();

        showModal(Modals.ConfirmSend, {
          amount: parsedAmount,
          denom: selectedDenom,
          to: address as Address,
          confirmSend,
          rejectSend,
        });

        await promise.catch(abort);

        await signingClient.transfer({
          transfer: {
            [address]: {
              [selectedCoin.denom]: parsedAmount.toString(),
            },
          },
          sender: account!.address as Address,
        });
      },
      onSuccess: () => {
        reset();
        refreshBalances();
        queryClient.invalidateQueries({ queryKey: ["quests", account?.username] });
      },
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
            <div className="flex flex-col w-full gap-4">
              <form onSubmit={handleSubmit(onSubmit)}>
                <div className="flex flex-col w-full gap-4">
                  <AssetInputWithRange
                    name="amount"
                    label="You're sending"
                    asset={selectedCoin}
                    balances={balances}
                    controllers={controllers}
                    isDisabled={isPending}
                    shouldValidate
                    showRange
                    showCoinSelector
                    onSelectCoin={(denom) => setSelectedDenom(denom)}
                    renderSelector={({ value, onChange, isDisabled }) => (
                      <CoinSelector
                        coins={
                          isConnected
                            ? Object.keys({ ...balances, "bridge/usdc": "" }).map(
                                (denom) => coins.byDenom[denom],
                              )
                            : [coins.byDenom[selectedDenom]]
                        }
                        value={value}
                        isDisabled={isDisabled}
                        onChange={(k) => onChange(k)}
                      />
                    )}
                  />
                  <AccountSearchInput
                    {...register("address", {
                      validate: (v) =>
                        isValidAddress(v) || m["errors.validations.invalidAddress"](),
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
              {showAdressWarning && <WarningTransferAccounts variant="send" />}
            </div>
          ) : (
            <div className="flex flex-col w-full gap-4">
              <div className="flex flex-col w-full gap-6 items-center justify-center text-center pb-10 bg-surface-secondary-rice rounded-xl shadow-account-card p-4">
                <div className="flex flex-col gap-1 items-center">
                  <p className="exposure-h3-italic">{`${capitalize((account?.type as string) || "")} Account #${account?.index}`}</p>
                  <div className="flex gap-1">
                    <TruncateText
                      className="diatype-sm-medium text-ink-tertiary-500"
                      text={account?.address}
                    />
                    <TextCopy
                      copyText={account?.address}
                      className="w-4 h-4 cursor-pointer text-ink-tertiary-500"
                    />
                  </div>
                </div>
                <QRCode data={account?.address as string} />
              </div>
              <WarningTransferAccounts variant="receive" />
            </div>
          )}
        </ResizerContainer>
      </div>
    </div>
  );
}
