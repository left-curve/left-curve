import { Button, Input, useInputs, useWizard } from "@left-curve/applets-kit";
import { capitalize, formatNumber, formatUnits, parseUnits } from "@left-curve/dango/utils";
import { useAccount, useBalances, useConfig, useSigningClient } from "@left-curve/store";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { m } from "~/paraglide/messages";

import type { AccountTypes } from "@left-curve/dango/types";
import type React from "react";

import { useApp } from "~/hooks/useApp";
import { Modals } from "../foundation/RootModal";
import { toast } from "../foundation/Toast";

export const CreateAccountDepositStep: React.FC = () => {
  const { done, previousStep, data } = useWizard<{ accountType: AccountTypes }>();
  const { register, inputs } = useInputs({ initialValues: { amount: "0" } });

  const { value: fundsAmount, error } = inputs.amount || {};

  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const { showModal } = useApp();
  const { coins, state } = useConfig();
  const { account, refreshAccounts, changeAccount } = useAccount();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { data: signingClient } = useSigningClient();

  const { data: balances = {}, refetch: refreshBalances } = useBalances({
    address: account?.address,
  });

  const { accountType } = data;
  const coinInfo = coins[state.chainId]["hyp/eth/usdc"];
  const humanBalance = formatUnits(balances["hyp/eth/usdc"] || 0, coinInfo.decimals);

  const { mutateAsync: send, isPending } = useMutation({
    mutationFn: async () => {
      if (!signingClient) throw new Error("error: no signing client");
      const parsedAmount = parseUnits(fundsAmount, coinInfo.decimals).toString();

      const [nextIndex] = await Promise.all([
        signingClient.getNextAccountIndex({ username: account!.username }),
      ]);

      await signingClient.registerAccount({
        sender: account!.address,
        config: { [accountType as "spot"]: { owner: account!.username } },
        funds: {
          "hyp/eth/usdc": parsedAmount,
        },
      });

      return {
        amount: parsedAmount,
        accountType,
        accountName: `${account!.username} ${capitalize(accountType)} #${nextIndex}`,
        denom: "hyp/eth/usdc",
      };
    },
    onSuccess: async (data) => {
      showModal(Modals.ConfirmAccount, data);
      await refreshAccounts?.();
      await refreshBalances();
      // changeAccount?.(data.address);
      queryClient.invalidateQueries({ queryKey: ["quests", account] });
      navigate({ to: "/" });
    },
    onError: (e) => {
      console.error(e);
      toast.error(
        {
          title: m["signup.errors.couldntCompleteRequest"]() as string,
          description: e instanceof Error ? e.message : e,
        },
        {
          duration: Number.POSITIVE_INFINITY,
        },
      );
    },
  });

  return (
    <form
      className="flex flex-col gap-6 w-full"
      onSubmit={(e) => {
        e.preventDefault();
        send();
      }}
    >
      <Input
        isDisabled={isPending}
        placeholder="0"
        {...register("amount", {
          strategy: "onChange",
          validate: (v) => {
            if (Number(v) > Number(humanBalance))
              return m["validations.errors.insufficientFunds"]();
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
            <img src={coinInfo.logoURI} className="w-5 h-5" alt={coinInfo.name} />
            <span className="diatype-m-regular text-gray-500 pt-1">{coinInfo.symbol}</span>
          </div>
        }
        bottomComponent={
          <div className="w-full flex justify-between">
            <p>{m["common.available"]()}</p>
            <p className="flex gap-1">
              <span>{coinInfo.symbol}</span>
              <span>{formatNumber(humanBalance, formatNumberOptions)}</span>
            </p>
          </div>
        }
      />
      <div className="flex gap-4">
        <Button type="button" fullWidth onClick={() => previousStep()} isDisabled={isPending}>
          {m["common.back"]()}
        </Button>
        <Button type="submit" fullWidth isLoading={isPending} isDisabled={!!error}>
          {m["common.continue"]()}
        </Button>
      </div>
    </form>
  );
};
