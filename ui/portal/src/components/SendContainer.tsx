import {
  AccountSearchInput,
  CoinSelector,
  DangoButton,
  GradientContainer,
  Input,
} from "@dango/shared";
import { useAccount, useBalances, useConfig, useSigningClient } from "@leftcurve/react";
import { isValidAddress } from "@leftcurve/sdk";
import type { Address } from "@leftcurve/types";
import { formatUnits, parseUnits } from "@leftcurve/utils";
import { useForm } from "react-hook-form";

export const SendContainer: React.FC = () => {
  const { coins: chainCoins } = useConfig();
  const { chainId, account } = useAccount();
  const { data: signingClient } = useSigningClient();

  const coins = chainCoins[chainId as string];
  const arrayOfCoins = Object.values(coins);

  const { register, watch, setValue, setError, handleSubmit, formState } = useForm<{
    amount: string;
    denom: string;
    address: string;
  }>({
    mode: "onChange",
    defaultValues: {
      denom: arrayOfCoins[0].denom,
    },
  });

  const { data: balances } = useBalances({ address: account?.address });

  const denom = watch("denom");
  const humanAmount = formatUnits(balances?.[denom] || 0, coins[denom].decimals);

  const { errors, isSubmitting } = formState;

  const onSubmit = handleSubmit(async (formData) => {
    if (!signingClient) throw new Error("error: no signing client");
    if (!isValidAddress(formData.address as Address)) {
      return setError("address", { message: "Invalid address" });
    }
    const coin = coins[formData.denom];

    const amount = parseUnits(formData.amount, coin.decimals);

    await signingClient.transfer({
      to: formData.address as Address,
      sender: account!.address as Address,
      coins: {
        [formData.denom]: amount.toString(),
      },
    });
  });

  return (
    <form className="w-full" onSubmit={onSubmit}>
      <GradientContainer className="gap-4 justify-center w-full min-h-[37.5rem]">
        <div className="p-6 rounded-full bg-surface-rose-200">
          <img src="/images/send-and-receive.webp" alt="transfer" className="w-[120px] h-[120px]" />
        </div>
        <div className="flex flex-col gap-6 w-full">
          <div className="w-full flex flex-col gap-6 p-3 bg-surface-rose-200 rounded-[20px]">
            <div className="flex flex-col gap-2">
              <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
                You send
              </p>
              <Input
                {...register("amount", {
                  validate: (v) => {
                    if (!v) return "Amount is required";
                    if (Number(v) <= 0) return "Amount must be greater than 0";
                    if (Number(v) > Number(humanAmount)) return "Insufficient balance";
                    return true;
                  },
                })}
                isDisabled={isSubmitting}
                placeholder="0"
                classNames={{ input: "text-3xl", inputWrapper: "py-4 pl-6 pr-4" }}
                error={errors.amount?.message}
                bottomComponent={
                  <div className="w-full items-center justify-between px-6 text-typography-rose-600 text-xs flex font-bold uppercase tracking-widest my-2">
                    <p>Balance:</p>
                    <p>
                      {humanAmount} {coins[denom].symbol}
                    </p>
                  </div>
                }
                endContent={
                  <CoinSelector
                    label="coins"
                    isDisabled={isSubmitting}
                    coins={arrayOfCoins}
                    selectedKey={denom}
                    onSelectionChange={(k) => setValue("denom", k.toString())}
                  />
                }
              />
            </div>
            <div className="flex flex-col gap-2">
              <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
                To
              </p>
              <AccountSearchInput
                name="address"
                disabled={isSubmitting}
                error={errors.address?.message}
                value={watch("address", "")}
                onChange={(v) => setValue("address", v)}
              />
            </div>
          </div>
          <DangoButton isLoading={isSubmitting}>Send</DangoButton>
        </div>
      </GradientContainer>
    </form>
  );
};
