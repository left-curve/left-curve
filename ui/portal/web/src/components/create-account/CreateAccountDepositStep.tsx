import { Button, Input, useInputs, useWizard } from "@left-curve/applets-kit";
import { formatNumber, formatUnits, parseUnits, wait } from "@left-curve/dango/utils";
import {
  useAccount,
  useBalances,
  useChainId,
  useConfig,
  useSigningClient,
} from "@left-curve/store-react";
import { useMutation } from "@tanstack/react-query";
import { m } from "~/paraglide/messages";

import type { AccountTypes } from "@left-curve/dango/types";
import type React from "react";
import { useApp } from "~/hooks/useApp";
import { useToast } from "../Toast";

export const CreateAccountDepositStep: React.FC = () => {
  const { done, previousStep, data } = useWizard<{ accountType: AccountTypes }>();
  const { register, inputs } = useInputs();

  const { value: fundsAmount } = inputs.amount || {};

  const config = useConfig();
  const chainId = useChainId();
  const { toast } = useToast();
  const { account, refreshAccounts } = useAccount();
  const { formatNumberOptions } = useApp();
  const { data: signingClient } = useSigningClient();

  const { data: balances = {}, refetch: refreshBalances } = useBalances({
    address: account?.address,
  });

  const { accountType } = data;
  const coins = config.coins[chainId];
  const usdcInfo = coins["hyp/eth/usdc"];
  const humanBalance = formatUnits(balances["hyp/eth/usdc"] || 0, usdcInfo.decimals);

  const { mutateAsync: send } = useMutation({
    mutationFn: async () => {
      if (!signingClient) throw new Error("error: no signing client");
      await signingClient.registerAccount(
        {
          sender: account!.address,
          config: { [accountType as "spot"]: { owner: account!.username } },
        },
        {
          funds: {
            "hyp/eth/usdc": parseUnits(fundsAmount, usdcInfo.decimals).toString(),
          },
        },
      );
      await wait(3000);
      toast.success({ title: "Account created" });
      await refreshAccounts?.();
    },
    onSuccess: () => [refreshBalances(), done()],
  });

  return (
    <div className="flex flex-col gap-6 w-full">
      <Input
        placeholder="0"
        {...register("amount", {
          validate: (v) => {
            if (Number(v) > Number(humanBalance)) return "Insufficient balance";
            return true;
          },
          mask: (v, prev) => {
            const regex = /^\d+(\.\d{0,18})?$/;
            if (v === "" || regex.test(v)) return v;
            return prev;
          },
        })}
        endContent={
          <div className="flex flex-row items-center gap-1 justify-center">
            <img src={usdcInfo.logoURI} className="w-5 h-5 rounded-full" alt={usdcInfo.name} />
            <span className="diatype-m-regular text-gray-500 pt-1">{usdcInfo.symbol}</span>
          </div>
        }
        bottomComponent={
          <div className="w-full flex justify-between">
            <p>{m["common.available"]()}</p>
            <p className="flex gap-1">
              <span>{usdcInfo.symbol}</span>
              <span>{formatNumber(humanBalance, formatNumberOptions)}</span>
            </p>
          </div>
        }
      />
      <div className="flex gap-4">
        <Button fullWidth onClick={() => previousStep()}>
          {m["common.back"]()}
        </Button>
        <Button fullWidth onClick={() => send()}>
          {m["common.continue"]()}
        </Button>
      </div>
    </div>
  );
};
