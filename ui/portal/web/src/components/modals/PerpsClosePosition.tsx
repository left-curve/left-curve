import {
  Button,
  IconButton,
  IconClose,
  Input,
  Range,
  numberMask,
  useApp,
} from "@left-curve/applets-kit";

import { useAccount, useConfig, useSigningClient, useSubmitTx } from "@left-curve/store";
import { PERPS_DEFAULT_SLIPPAGE } from "~/constants";
import { useQueryClient } from "@tanstack/react-query";
import { forwardRef, useCallback, useMemo, useState } from "react";
import { Decimal } from "@left-curve/dango/utils";

import { m } from "@left-curve/foundation/paraglide/messages.js";

type PerpsClosePositionProps = {
  pairId: string;
  size: string;
  pnl: number;
};

export const PerpsClosePosition = forwardRef<void, PerpsClosePositionProps>(({ pairId, size }) => {
  const { hideModal } = useApp();
  const { account } = useAccount();
  const { coins } = useConfig();
  const { data: signingClient } = useSigningClient();
  const queryClient = useQueryClient();

  const sizeNum = Math.abs(Number(size));
  const isLong = Number(size) > 0;

  const baseSymbol = pairId.replace("perp/", "").replace(/usd$/i, "");
  const baseCoin = Object.values(coins.byDenom).find((c) => c.symbol.toLowerCase() === baseSymbol);
  const symbol = baseCoin?.symbol ?? baseSymbol.toUpperCase();
  const logoURI = baseCoin?.logoURI;

  const [inputValue, setInputValue] = useState(sizeNum.toString());
  const closeAmount = Math.min(Number(inputValue) || 0, sizeNum);

  const percentage = useMemo(() => {
    if (sizeNum === 0) return 0;
    return Math.round((closeAmount / sizeNum) * 100);
  }, [closeAmount, sizeNum]);

  const handlePercentageChange = useCallback(
    (pct: number) => {
      const rounded = Math.round(pct);
      const amount =
        rounded === 100 ? sizeNum : Number(Decimal(sizeNum).mul(rounded).div(100).toFixed(4));
      setInputValue(amount.toString());
    },
    [sizeNum],
  );

  const handleInputChange = useCallback(
    (value: string) => {
      setInputValue(numberMask(value, inputValue));
    },
    [inputValue],
  );

  const closeSize = isLong ? `-${closeAmount}` : `${closeAmount}`;

  const { isPending, mutateAsync: closePosition } = useSubmitTx({
    submission: {
      success: "Position closed successfully",
    },
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("No signing client available");
        await signingClient.submitPerpsOrder({
          sender: account!.address,
          pairId,
          size: closeSize,
          kind: { market: { maxSlippage: PERPS_DEFAULT_SLIPPAGE } },
          reduceOnly: true,
        });
      },
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["prices"] });
        queryClient.invalidateQueries({ queryKey: ["perpsTradeHistory", account?.address] });
        hideModal();
      },
    },
  });

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-6 w-full md:max-w-[28rem]">
      <IconButton
        className="hidden md:block absolute right-4 top-4"
        variant="link"
        onClick={() => hideModal()}
      >
        <IconClose />
      </IconButton>

      <div className="flex flex-col gap-2">
        <h2 className="text-ink-primary-900 diatype-lg-bold">{m["modals.marketClose.title"]()}</h2>
        <p className="text-ink-tertiary-500 diatype-sm-regular">
          {m["modals.marketClose.description"]()}
        </p>
      </div>

      <div className="flex flex-col gap-1">
        <div className="flex justify-between">
          <p className="diatype-sm-regular text-ink-tertiary-500">
            {m["modals.marketClose.size"]()}
          </p>
          <p className="diatype-sm-medium text-primitives-red-light-400">
            {sizeNum} {symbol}
          </p>
        </div>
        <div className="flex justify-between">
          <p className="diatype-sm-regular text-ink-tertiary-500">
            {m["modals.marketClose.price"]()}
          </p>
          <p className="diatype-sm-medium text-ink-secondary-700">
            {m["modals.marketClose.market"]()}
          </p>
        </div>
      </div>

      <div className="flex flex-col gap-3">
        <Input
          placeholder="0"
          label={m["modals.marketClose.size"]()}
          value={inputValue}
          onChange={(e) => handleInputChange(e.target.value)}
          classNames={{
            inputWrapper: "pl-0 py-3 h-auto gap-[6px]",
            inputParent: "h-[34px] diatype-lg-medium min-w-0",
            input: "!diatype-lg-medium",
          }}
          startText="right"
          startContent={
            <div className="flex items-center gap-2 pl-4">
              {logoURI && <img src={logoURI} alt={symbol} className="w-8 h-8 rounded-full" />}
              <p className="text-ink-tertiary-500 diatype-lg-medium">{symbol}</p>
            </div>
          }
        />
        <Range
          minValue={0}
          maxValue={100}
          step={1}
          value={percentage}
          onChange={handlePercentageChange}
          withInput
          inputEndContent="%"
          showSteps={[
            { value: 0, label: "0%" },
            { value: 25, label: "25%" },
            { value: 50, label: "50%" },
            { value: 75, label: "75%" },
            { value: 100, label: "100%" },
          ]}
        />
      </div>

      <Button
        fullWidth
        isLoading={isPending}
        onClick={() => closePosition()}
        isDisabled={closeAmount <= 0}
      >
        {m["modals.marketClose.confirm"]()}
      </Button>
    </div>
  );
});
