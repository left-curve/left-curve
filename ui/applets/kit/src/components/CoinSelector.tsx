import { Select } from "./Select";
import { twMerge } from "@left-curve/foundation";

import type { AnyCoin } from "@left-curve/store/types";
import type { SelectProps } from "./Select";

interface Props extends Omit<SelectProps, "children"> {
  coins: AnyCoin[];
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

export const CoinSelector: React.FC<Props> = ({ coins, defaultValue, classNames, ...props }) => {
  return (
    <Select
      defaultValue={defaultValue || coins[0].denom}
      classNames={{
        base: `${classNames?.base}`,
        listboxWrapper: `top-12 ${classNames?.listboxWrapper}`,
        listbox: `${classNames?.listbox}`,
        value: `${classNames?.value}`,
        trigger: `min-w-10 p-3 bg-transparent shadow-none justify-start ${classNames?.trigger}`,
        icon: `${classNames?.selectorIcon}`,
      }}
      {...props}
    >
      {coins.map((coin) => (
        <Select.Item key={coin.denom} value={coin.denom}>
          <div className="flex gap-2 items-center font-semibold">
            <img
              src={coin.logoURI}
              alt={coin.symbol}
              className={twMerge("w-8 h-8", classNames?.coin)}
            />
            <p>{coin.symbol}</p>
          </div>
        </Select.Item>
      ))}
    </Select>
  );
};
