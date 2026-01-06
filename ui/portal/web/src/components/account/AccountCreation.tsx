import { Modals, useApp, useInputs, useMediaQuery } from "@left-curve/applets-kit";
import {
  useAccount,
  useBalances,
  useConfig,
  useSigningClient,
  useSubmitTx,
} from "@left-curve/store";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect } from "react";

import { Decimal, formatNumber, formatUnits, parseUnits } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  Button,
  IconButton,
  IconChevronDown,
  Input,
  ResizerContainer,
} from "@left-curve/applets-kit";

import type React from "react";

export const Container: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { isMd } = useMediaQuery();
  const { history } = useRouter();

  return (
    <div className="flex items-center justify-start w-full h-full flex-col md:max-w-[360px] text-center gap-8">
      <div className="flex flex-col gap-4 items-center justify-center w-full">
        <div className="flex flex-col gap-1 items-center justify-center w-full">
          <h2 className="flex gap-2 items-center justify-center w-full relative">
            {isMd ? null : (
              <IconButton
                variant="link"
                onClick={() => history.go(-1)}
                className="absolute left-0 top-0"
              >
                <IconChevronDown className="rotate-90" />
              </IconButton>
            )}
            <span className="h2-heavy">{m["accountCreation.title"]()}</span>
          </h2>
          <p className="text-ink-tertiary-500 diatype-m-medium">
            {m["accountCreation.description"]()}
          </p>
        </div>
      </div>
      <ResizerContainer layoutId="create-account" className="w-full max-w-[22.5rem]">
        {children}
      </ResizerContainer>
    </div>
  );
};

export const Deposit: React.FC = () => {
  const { register, inputs } = useInputs();
  const navigate = useNavigate();

  const { value: fundsAmount, error } = inputs.amount || {};

  const { toast, showModal, subscriptions, settings } = useApp();
  const { coins } = useConfig();
  const { username, userIndex, account } = useAccount();
  const { formatNumberOptions } = settings;
  const { data: signingClient } = useSigningClient();

  const { data: balances = {} } = useBalances({
    address: account?.address,
  });

  const coinInfo = coins.byDenom["bridge/usdc"];
  const humanBalance = formatUnits(balances["bridge/usdc"] || 0, coinInfo.decimals);

  const { mutateAsync: send, isPending } = useSubmitTx({
    toast: {
      error: () =>
        toast.error({
          title: m["common.error"](),
          description: m["signup.errors.couldntCompleteRequest"](),
        }),
    },
    submission: {
      success: m["accountCreation.accountCreated"](),
      error: m["signup.errors.couldntCompleteRequest"](),
    },
    mutation: {
      invalidateKeys: [["quests", userIndex]],
      mutationFn: async () => {
        if (!signingClient) throw new Error("error: no signing client");
        const funds = fundsAmount || "0";

        const parsedAmount = parseUnits(funds, coinInfo.decimals);

        await signingClient.registerAccount({
          sender: account!.address,
          config: { single: { owner: userIndex as number } },
          ...(Decimal(funds).gt(0) ? { funds: { "bridge/usdc": parsedAmount.toString() } } : {}),
        });
      },
    },
  });

  useEffect(() => {
    if (!account) return;
    return subscriptions.subscribe("account", {
      params: { userIndex: userIndex as number },
      listener: async ({ accounts }) => {
        const account = accounts.at(0)!;
        const parsedAmount = parseUnits(fundsAmount || "0", coinInfo.decimals).toString();

        showModal(Modals.ConfirmAccount, {
          navigate,
          amount: parsedAmount,
          accountAddress: account.address,
          accountType: account.accountType,
          accountName: `Account #${account.accountIndex}`,
          denom: "bridge/usdc",
        });
      },
    });
  }, [subscriptions, username, fundsAmount, coinInfo]);

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
              return m["errors.validations.insufficientFunds"]();
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
            <img src={coinInfo.logoURI} className="w-5 h-5" alt={coinInfo.symbol} />
            <span className="diatype-m-regular text-ink-tertiary-500 pt-1">{coinInfo.symbol}</span>
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
        <Button type="submit" fullWidth isLoading={isPending} isDisabled={!!error}>
          {m["common.continue"]()}
        </Button>
      </div>
    </form>
  );
};

export const AccountCreation = Object.assign(Container, {
  Deposit,
});
