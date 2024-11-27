import { Button, Input, useWizard } from "@dango/shared";
import { useAccount, useBalances, useConfig } from "@left-curve/react";
import { motion } from "framer-motion";

import type { AccountTypes, NativeCoin } from "@left-curve/types";
import { formatUnits, parseUnits, wait } from "@left-curve/utils";
import { useForm } from "react-hook-form";

import type { SignerClient } from "@left-curve/sdk/clients";
import { useNavigate } from "react-router-dom";

export const TransferStep: React.FC = () => {
  const navigate = useNavigate();
  const { chains, coins } = useConfig();
  const { account, connector, refreshAccounts } = useAccount();
  const { data } = useWizard<{ accountType: AccountTypes }>();
  const { register, formState, setValue, handleSubmit, watch } = useForm<{ amount: string }>({
    mode: "onChange",
  });

  const { errors, isSubmitting } = formState;
  const chain = chains.at(0)!;
  const chainCoins = coins[chain.id];
  const { logoURI, symbol, denom, decimals } = chainCoins[chain.nativeCoin.denom] as NativeCoin;

  const { data: balances = {} } = useBalances({ address: account?.address });
  const humanBalance = formatUnits(balances[denom] || 0, decimals);

  const onSubmit = handleSubmit(async ({ amount }) => {
    const client: SignerClient = await connector?.getClient();
    await client.registerAccount(
      {
        sender: account!.address,
        config: { [data.accountType as "spot"]: { owner: account!.username } },
      },
      {
        funds: {
          [denom]: parseUnits(amount, decimals).toString(),
        },
      },
    );
    await wait(1000);
    await refreshAccounts?.();
    navigate("/?showAccounts=true");
  });

  return (
    <motion.form
      onSubmit={onSubmit}
      className="flex flex-col w-full justify-center gap-16"
      initial={{ translateY: -100 }}
      animate={{ translateY: 0 }}
      exit={{ translateY: 100 }}
    >
      <div className="flex flex-col gap-4 items-center text-center w-full">
        <h3 className="text-typography-black-200 font-extrabold text-lg tracking-widest uppercase">
          Transfer Assets
        </h3>
        <p className="text-typography-black-100 text-xl">
          Fund your account with assets from your current account.
        </p>
      </div>
      <div className="flex flex-col gap-6 w-full">
        <div className="p-3 bg-surface-rose-200 w-full rounded-[20px] flex items-center justify-center gap-6">
          <img
            src={`/images/avatars/${data.accountType}.svg`}
            alt="account-avatar"
            className="h-16 w-16"
          />
          <div className="w-full flex flex-col gap-2">
            <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
              Deposit amount
            </p>
            <Input
              {...register("amount", {
                validate: (v) => {
                  if (Number(v) > Number(humanBalance)) return "Insufficient balance";
                  return true;
                },
              })}
              onChange={({ target }) => {
                const regex = /^\d+(\.\d{0,18})?$/;
                if (target.value === "" || regex.test(target.value)) {
                  setValue("amount", target.value, { shouldValidate: true });
                }
              }}
              value={watch("amount", "")}
              startText="right"
              placeholder="0"
              disabled={isSubmitting}
              error={errors.amount?.message?.toString()}
              startContent={
                <div className="flex flex-row items-center gap-2">
                  <img
                    src={logoURI}
                    className="w-8 h-8 rounded-full"
                    alt="logo-chain-native-coin"
                  />
                  <span className="text-typography-black-100 inline-block">{symbol}</span>
                </div>
              }
              bottomComponent={
                <div className="px-6 w-full flex justify-between text-typography-rose-500 uppercase font-semibold text-[12px]">
                  <p>Available</p>
                  <p className="flex gap-1">
                    <span>{symbol}</span>
                    <span>{humanBalance}</span>
                  </p>
                </div>
              }
            />
          </div>
        </div>

        <div className="flex flex-col gap-1 w-full items-center justify-center">
          <Button color="rose" fullWidth isLoading={isSubmitting}>
            Create Account
          </Button>
        </div>
      </div>
    </motion.form>
  );
};
