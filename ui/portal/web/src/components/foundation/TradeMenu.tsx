import { useRouter } from "@tanstack/react-router";
import { Sheet } from "react-modal-sheet";
import { useApp } from "~/hooks/useApp";

import { motion } from "framer-motion";

import { m } from "~/paraglide/messages";

import { IconButton, IconChevronDown, IconUser, Tabs, twMerge } from "@left-curve/applets-kit";
import { AnimatePresence } from "framer-motion";

import { act, useState, type PropsWithChildren } from "react";
import type React from "react";

const Root: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type TradeMenuProps = {
  backAllowed?: boolean;
};

const Menu: React.FC<TradeMenuProps> = ({ backAllowed }) => {
  const { isTradeBarVisible, setTradeBarVisibility, setSidebarVisibility } = useApp();
  const { history } = useRouter();
  const [action, setAction] = useState<"sell" | "buy">("sell");

  if (!isTradeBarVisible) return null;

  return (
    <>
      <div className="w-full flex items-center flex-col gap-6 relative md:pt-4">
        <div className="w-full flex items-center justify-between px-4 gap-2">
          <IconButton
            variant="utility"
            size="lg"
            type="button"
            onClick={() => setTradeBarVisibility(false)}
          >
            <IconChevronDown className="h-6 w-6" />
          </IconButton>
          <Tabs
            layoutId="tabs-sell-and-buy"
            selectedTab={action}
            keys={["sell", "buy"]}
            fullWidth
            onTabChange={() => setAction(action === "sell" ? "buy" : "sell")}
            color={action === "sell" ? "red" : "green"}
          />
          <IconButton
            variant="utility"
            size="lg"
            type="button"
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
