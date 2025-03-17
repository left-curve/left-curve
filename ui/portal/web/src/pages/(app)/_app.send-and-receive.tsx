import { capitalize, formatUnits, parseUnits, wait } from "@left-curve/dango/utils";
import {
  useAccount,
  useBalances,
  useChainId,
  useConfig,
  useSigningClient,
} from "@left-curve/store-react";
import { createFileRoute, useNavigate, useSearch } from "@tanstack/react-router";
import { useState } from "react";

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

  const [selectedDenom, setSelectedDenom] = useState("uusdc");
  const { register, setError, setValue, inputs, reset } = useInputs();
  const { address, amount } = inputs;

  const { account, isConnected } = useAccount();
  const chainId = useChainId();
  const config = useConfig();
  const { data: signingClient } = useSigningClient();

  const { data: balances = {}, refetch: refreshBalances } = useBalances({
    address: account?.address,
  });

  const coins = config.coins[chainId];
  const selectedCoin = coins[selectedDenom];

  const humanAmount = formatUnits(balances[selectedDenom] || 0, selectedCoin.decimals);

  const { mutateAsync: send, isPending } = useMutation({
    mutationFn: async () => {
      if (!amount?.value) return setError("amount", "Amount is required");
      if (!address?.value) return setError("address", "Address is required");
      if (!signingClient) throw new Error("error: no signing client");
      if (!isValidAddress(address.value)) {
        return setError("address", "Invalid address");
      }

      await signingClient.transfer({
        to: address.value as Address,
        sender: account!.address as Address,
        coins: {
          [selectedCoin.denom]: parseUnits(amount.value, selectedCoin.decimals).toString(),
        },
      });
      await wait(1000);
    },
    onSuccess: () => {
      reset();
      refreshBalances();
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
            selectedTab={isConnected ? action : "send"}
            keys={isConnected ? ["send", "receive"] : ["send"]}
            fullWidth
            onTabChange={(v) => navigate({ search: { action: v }, replace: false })}
          />

          {action === "send" ? (
            <>
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
                    validate: (v) => {
                      if (!v) return "Amount is required";
                      if (Number(v) <= 0) return "Amount must be greater than 0";
                      if (Number(v) > Number(humanAmount)) return "Insufficient funds";
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
                      onSelectionChange={(k) => setSelectedDenom(k.toString())}
                    />
                  }
                  insideBottomComponent={
                    <div className="w-full flex justify-between pl-4 h-[22px]">
                      <div className="flex gap-1 items-center justify-center diatype-sm-regular text-gray-500">
                        <span>{selectedCoin.symbol}</span>
                        <span>{humanAmount}</span>
                        <Button
                          isDisabled={isPending}
                          variant="secondary"
                          size="xs"
                          className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                          onClick={() => setValue("amount", humanAmount)}
                        >
                          Max
                        </Button>
                      </div>
                      <p>{humanAmount}</p>
                    </div>
                  }
                />
                <AccountSearchInput
                  {...register("address")}
                  label="To"
                  placeholder="Wallet address or name"
                  onChange={(v) => setValue("address", v)}
                  isDisabled={isPending}
                />
              </div>

              <Button
                fullWidth
                className="mt-5"
                onClick={() => send()}
                isLoading={isPending}
                isDisabled={!isConnected}
              >
                Send
              </Button>
            </>
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
