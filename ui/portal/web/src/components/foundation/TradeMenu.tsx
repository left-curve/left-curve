import { Sheet } from "react-modal-sheet";
import { useApp } from "~/hooks/useApp";

import {
  Button,
  Checkbox,
  IconButton,
  IconChevronDown,
  IconUser,
  Input,
  Tabs,
} from "@left-curve/applets-kit";

import { useState, type PropsWithChildren } from "react";
import type React from "react";

import { m } from "~/paraglide/messages";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type TradeMenuProps = {
  action?: "sell" | "buy";
};

export const Menu: React.FC<TradeMenuProps> = ({ action: defaultAction }) => {
  const { setTradeBarVisibility, setSidebarVisibility } = useApp();
  const [action, setAction] = useState<"sell" | "buy">(defaultAction || "buy");
  const [operation, setOperation] = useState<"market" | "limit">("limit");

  return (
    <div className="w-full flex items-center flex-col gap-4 relative">
      <div className="w-full flex items-center justify-between px-4 gap-2">
        <IconButton
          variant="utility"
          size="lg"
          type="button"
          className="lg:hidden"
          onClick={() => setTradeBarVisibility(false)}
        >
          <IconChevronDown className="h-6 w-6" />
        </IconButton>
        <Tabs
          layoutId="tabs-sell-and-buy"
          selectedTab={action}
          keys={["buy", "sell"]}
          fullWidth
          onTabChange={(tab) => setAction(tab as "sell" | "buy")}
          color={action === "sell" ? "red" : "green"}
        />
        <IconButton
          variant="utility"
          size="lg"
          type="button"
          className="lg:hidden"
          onClick={() => [setTradeBarVisibility(false), setSidebarVisibility(true)]}
        >
          <IconUser className="h-6 w-6" />
        </IconButton>
      </div>
      <div className="w-full flex flex-col gap-4 p-4">
        <Tabs
          layoutId="tabs-market-limit"
          selectedTab={operation}
          keys={["market", "limit"]}
          fullWidth
          onTabChange={(tab) => setOperation(tab as "market" | "limit")}
          color="line-red"
        />
        <div className="flex items-center justify-between gap-2">
          <p className="diatype-xs-medium text-gray-500">Current Position</p>
          <p className="diatype-xs-bold text-gray-700">123.00 ETH</p>
        </div>
        <Input
          placeholder="0"
          label="Size"
          classNames={{
            base: "z-20",
            inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px]",
            inputParent: "h-[34px] h3-bold",
            input: "!h3-bold",
          }}
          startText="right"
          startContent={
            <div className="inline-flex flex-row items-center gap-3 diatype-m-regular h-[46px] rounded-md min-w-14 p-3 bg-transparent justify-start">
              <div className="flex gap-2 items-center font-semibold">
                <img
                  src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
                  alt="usdc"
                  className="w-8 h-8"
                />
                <p>USDC</p>
              </div>
            </div>
          }
          insideBottomComponent={
            <div className="flex items-center justify-between gap-2 w-full h-[22px] text-gray-500 diatype-sm-regular pl-4">
              <div className="flex items-center gap-2">
                <p>12.23</p>
                <Button
                  type="button"
                  variant="secondary"
                  size="xs"
                  className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                >
                  {m["common.max"]()}
                </Button>
              </div>
            </div>
          }
        />
        <Input placeholder="0" label="Price" endContent={<p>USDC</p>} />
        <Checkbox radius="md" size="sm" label="Take Profit/Stop Loss" />
        <div className="grid grid-cols-2 gap-2">
          <Input placeholder="0" label="TP Price" />
          <Input placeholder="0" label="TP Price" endContent="%" />
          <Input placeholder="0" label="SL Price" />
          <Input placeholder="0" label="Loss" endContent="%" />
        </div>
        <Button variant={action === "sell" ? "primary" : "tertiary"} fullWidth>
          Enable Trading
        </Button>
      </div>
    </div>
  );
};

export const Mobile: React.FC = () => {
  const { isTradeBarVisible, setTradeBarVisibility } = useApp();

  return (
    <Sheet isOpen={isTradeBarVisible} onClose={() => setTradeBarVisibility(false)} rootId="root">
      <Sheet.Container className="!bg-white-100 !rounded-t-2xl !shadow-none">
        <Sheet.Header />
        <Sheet.Content>
          <Menu />
        </Sheet.Content>
      </Sheet.Container>
      <Sheet.Backdrop onTap={() => setTradeBarVisibility(false)} />
    </Sheet>
  );
};

const ExportComponent = Object.assign(Container, {
  Mobile,
  Menu,
});

export { ExportComponent as TradeMenu };
