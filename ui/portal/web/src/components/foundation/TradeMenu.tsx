import { Sheet } from "react-modal-sheet";
import { useApp } from "~/hooks/useApp";

import { IconButton, IconChevronDown, IconUser, Tabs } from "@left-curve/applets-kit";

import { useState, type PropsWithChildren } from "react";
import type React from "react";

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
