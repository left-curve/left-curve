import { Sheet } from "react-modal-sheet";
import { useApp } from "~/hooks/useApp";

import { IconButton, IconChevronDown, IconUser, Tabs } from "@left-curve/applets-kit";

import { useState, type PropsWithChildren } from "react";
import type React from "react";

const Root: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type TradeMenuProps = {
  action?: "sell" | "buy";
};

const Menu: React.FC<TradeMenuProps> = ({ action: defaultAction }) => {
  const { isTradeBarVisible, setTradeBarVisibility, setSidebarVisibility } = useApp();
  const [action, setAction] = useState<"sell" | "buy">(defaultAction || "buy");

  if (!isTradeBarVisible) return null;

  return (
    <>
      <div className="w-full flex items-center flex-col gap-6 relative md:pt-4">
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
            onTabChange={() => setAction(action === "sell" ? "buy" : "sell")}
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
        <div className="w-full flex items-center justify-center gap-2">Trade menu content</div>
      </div>
    </>
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

const ExportComponent = Object.assign(Root, {
  Mobile,
});

export { ExportComponent as TradeMenu };
