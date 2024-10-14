import type { AnyCoin } from "@leftcurve/types";
import { Item, Select, type SelectProps } from "../atoms/Select";

interface Props extends Omit<SelectProps, "children"> {
  coins: AnyCoin[];
}

export const CoinSelector: React.FC<Props> = ({ coins, defaultSelectedKey, color, ...props }) => {
  return (
    <Select
      defaultSelectedKey={defaultSelectedKey || coins[0].denom}
      color={color || "white"}
      classNames={{
        base: "w-fit",
        value: "text-sm",
        trigger: "p-2",
        listboxWrapper: "px-2",
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
