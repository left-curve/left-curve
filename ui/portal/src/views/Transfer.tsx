import { CoinSelector, DangoButton, GradientContainer, Input, twMerge } from "@dango/shared";
import { useAccount, useBalances, useConfig } from "@leftcurve/react";
import type React from "react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useParams } from "react-router-dom";

const actions = ["send", "transfer"];

const Transfer: React.FC = () => {
  const { action: actionParam = "send" } = useParams<{ action: string }>();

  const { coins: chainCoins } = useConfig();
  const { chainId, account } = useAccount();
  const { formState, watch, setValue } = useForm<{ amount: string; denom: string }>();

  const coins = chainCoins[chainId as string];
  const arrayOfCoins = Object.values(coins);
  const denom = watch("denom", arrayOfCoins.at(0)?.denom);

  const { data: balance } = useBalances({ address: account?.address });

  const [action, setAction] = useState<string>(actionParam);

  return (
    <div className="min-h-full w-full flex-1 flex items-center justify-center z-10 relative p-4">
      <div className="flex flex-col gap-8 w-full items-center justify-center max-w-[38.5rem]">
        {/* Buttons Switcher */}
        <div className="w-full items-center justify-end flex">
          <div className="p-1 bg-surface-green-300 text-typography-green-300 rounded-2xl">
            {actions.map((act) => (
              <button
                key={act}
                type="button"
                className={twMerge(
                  "capitalize rounded-xl p-2 transition-all font-bold italic",
                  action.includes(act) ? "bg-surface-green-400 text-typography-green-400" : "",
                )}
                onClick={() => setAction(act)}
              >
                {act}
              </button>
            ))}
          </div>
        </div>
        {/* End Button Switcher */}
        <GradientContainer className="gap-4 justify-center w-full">
          <div className="p-6 rounded-full bg-surface-rose-200">
            <img
              src="/images/applets/send-and-receive.png"
              alt="transfer"
              className="w-[120px] h-[120px]"
            />
          </div>
          <div className="flex flex-col gap-6 w-full">
            {/* Inputs container */}
            <div className="w-full flex flex-col gap-6 p-3 bg-surface-rose-200 rounded-[20px]">
              <div className="flex flex-col gap-2">
                <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
                  You send
                </p>
                <Input
                  placeholder="0"
                  classNames={{ input: "text-3xl", inputWrapper: "py-4 pl-6 pr-4" }}
                  bottomComponent={
                    <div className="w-full items-center justify-between px-6 text-typography-rose-600 text-xs flex font-bold uppercase tracking-widest my-2">
                      <p>Balance:</p>
                      <p>
                        {balance?.[denom] || 0} {coins[denom].symbol}
                      </p>
                    </div>
                  }
                  endContent={
                    <CoinSelector
                      coins={arrayOfCoins}
                      defaultSelectedKey={denom}
                      onSelectionChange={(k) => setValue("denom", k.toString())}
                    />
                  }
                />
              </div>
              <div className="flex flex-col gap-2">
                <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
                  To
                </p>
                <Input placeholder="Uner name or wallet address" />
              </div>
            </div>
            {/* End Inputs container */}
            <DangoButton>Send</DangoButton>
          </div>
        </GradientContainer>
      </div>
    </div>
  );
};

export default Transfer;
