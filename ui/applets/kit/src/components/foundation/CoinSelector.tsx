import { Select } from "./Select";

import type { AnyCoin } from "@left-curve/dango/types";
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
  };
}

export const CoinSelector: React.FC<Props> = ({ coins, defaultValue, classNames, ...props }) => {
  return (
    <Select
      defaultValue={defaultValue || coins[0].denom}
      classNames={{
        trigger: `min-w-14 p-3 bg-transparent shadow-none justify-start ${classNames?.trigger}`,
        listboxWrapper: "top-12",
      }}
      {...props}
    >
      {coins.map((coin) => (
        <Select.Item key={coin.denom} value={coin.denom}>
          <div className="flex gap-2 items-center font-semibold">
            <img src={coin.logoURI} alt={coin.symbol} className="w-8 h-8" />
            <p>{coin.symbol}</p>
          </div>
        </Select.Item>
      ))}
    </Select>
  );
};
