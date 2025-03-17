import type { AnyCoin } from "@left-curve/dango/types";
import { Item, Select, type SelectProps } from "./Select";

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

export const CoinSelector: React.FC<Props> = ({
  coins,
  defaultSelectedKey,
  classNames,
  ...props
}) => {
  return (
    <Select
      defaultSelectedKey={defaultSelectedKey || coins[0].denom}
      classNames={{
        trigger: `p-2 bg-transparent shadow-none justify-center ${classNames?.trigger}`,
      }}
      {...props}
    >
      {coins.map((coin) => (
        <Item key={coin.denom} textValue={coin.denom}>
          <div className="flex gap-2 items-center font-semibold">
            <img src={coin.logoURI} alt={coin.symbol} className="w-8 h-8 rounded-full" />
            <p>{coin.symbol}</p>
          </div>
        </Item>
      ))}
    </Select>
  );
};
