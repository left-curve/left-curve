import { Select } from "./Select";
import { twMerge, useApp } from "@left-curve/foundation";

import type { AnyCoin } from "@left-curve/store/types";
import type { SelectProps } from "./Select";
import { usePrices } from "@left-curve/store";

export interface CoinSelectorProps extends Omit<SelectProps, "children"> {
  coins: AnyCoin[];
  variant?: "boxed" | "plain";
  withName?: boolean;
  withPrice?: boolean;
  classNames?: {
    base?: string;
    listboxWrapper?: string;
    listbox?: string;
    value?: string;
    selectorIcon?: string;
    trigger?: string;
    coin?: string;
  };
}

export const CoinSelector: React.FC<CoinSelectorProps> = ({
  coins,
  classNames,
  variant = "plain",
  withName,
  withPrice,
  ...props
}) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { getPrice } = usePrices();
  return (
    <Select
      classNames={{
        base: `${classNames?.base}`,
        listboxWrapper: `top-12 ${classNames?.listboxWrapper}`,
        listbox: `${classNames?.listbox}`,
        value: `${classNames?.value}`,
        trigger: `${variant === "plain" ? `min-w-10 p-3 bg-transparent shadow-none justify-start ${classNames?.trigger}` : `${classNames?.trigger}`}`,
        icon: `${classNames?.selectorIcon}`,
      }}
      {...props}
    >
      {coins.map((coin) => (
        <Select.Item key={coin.denom} value={coin.denom}>
          <div className="flex gap-2 items-center font-semibold w-full justify-between">
            <div className="flex gap-2 items-center">
              <img
                src={coin.logoURI}
                alt={coin.symbol}
                className={twMerge("w-8 h-8", classNames?.coin)}
              />
              <div className="flex flex-col">
                <p>{coin.symbol}</p>
                {withName && (
                  <p data-hide-in-trigger className="diatype-sm-regular text-ink-tertiary-500">
                    {coin.name}
                  </p>
                )}
              </div>
            </div>

            {withPrice && (
              <div data-hide-in-trigger className="flex flex-col gap-2">
                <p className="diatype-sm-medium text-ink-tertiary-500">
                  {getPrice(1, coin.denom, {
                    format: true,
                    formatOptions: formatNumberOptions,
                  })}
                </p>
              </div>
            )}
          </div>
        </Select.Item>
      ))}
    </Select>
  );
};
