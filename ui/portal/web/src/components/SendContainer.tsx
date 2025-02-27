import {
  AccountSearchInput,
  Button,
  CoinSelector,
  Input,
  useSigningClient,
} from "@left-curve/applets-kit";
import { isValidAddress } from "@left-curve/dango";
import type { Address } from "@left-curve/dango/types";
import { formatUnits, parseUnits, wait } from "@left-curve/dango/utils";
import { useAccount, useBalances, useConfig } from "@left-curve/store-react";
import { useForm } from "react-hook-form";

export const SendContainer: React.FC = () => {
  const { coins: chainCoins } = useConfig();
  const { chainId, account } = useAccount();
  const { data: signingClient } = useSigningClient();

  const coins = chainCoins[chainId as string];
  const arrayOfCoins = Object.values(coins);

  const { register, watch, setValue, setError, handleSubmit, formState, reset } = useForm<{
    amount: string;
    denom: string;
    address: string;
  }>({
    mode: "onChange",
    defaultValues: {
      denom: arrayOfCoins[0].denom,
    },
  });

  const { data: balances, refetch } = useBalances({ address: account?.address });

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
    await wait(1000);
    await refetch();
    reset();
  });

  return (
    <form className="w-full" onSubmit={onSubmit}>
      <div className="dango-grid-5x5-M gap-4 flex flex-col items-center w-full">
        <div className="p-4 rounded-full bg-surface-rose-200">
          <img src="/images/send-and-receive.webp" alt="transfer" className="w-[88px] h-[88px]" />
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
                value={watch("amount", "")}
                onChange={({ target }) => {
                  const regex = /^\d+(\.\d{0,18})?$/;
                  if (target.value === "" || regex.test(target.value)) {
                    setValue("amount", target.value, { shouldValidate: true });
                  }
                }}
                isDisabled={isSubmitting}
                placeholder="0"
                classNames={{ input: "text-3xl", inputWrapper: "py-4 pl-6 pr-4" }}
                errorMessage={errors.amount?.message}
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
                errorMessage={errors.address?.message}
                value={watch("address", "")}
                onChange={(v) => setValue("address", v)}
              />
            </div>
          </div>
          <Button isLoading={isSubmitting}>Send</Button>
        </div>
      </div>
    </form>
  );
};
