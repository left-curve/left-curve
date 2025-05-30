import { Button, IconButton, IconChevronRight, twMerge } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import type React from "react";
import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";

export const TradeButtons: React.FC = () => {
  const navigate = useNavigate();
  const { setTradeBarVisibility } = useApp();
  const { isConnected } = useAccount();

  return (
    <div className="flex gap-2 items-center justify-center w-full">
      <IconButton
        variant="utility"
        size="lg"
        type="button"
        className={twMerge("shadow-btn-shadow-gradient")}
        onClick={() => navigate({ to: "/" })}
      >
        <IconChevronRight className="h-6 w-6 rotate-180 " />
      </IconButton>
      {isConnected ? (
        <div className="flex-1 flex gap-2">
          <Button
            className="h-full"
            fullWidth
            variant="tertiary"
            onClick={() => setTradeBarVisibility(true)}
          >
            {m["proSwap.buy"]()}
          </Button>
          <Button className="h-full" fullWidth onClick={() => setTradeBarVisibility(true)}>
            {m["proSwap.sell"]()}
          </Button>
        </div>
      ) : (
        <Button className="flex-1 h-full" onClick={() => navigate({ to: "/signin" })}>
          Connect
        </Button>
      )}
    </div>
  );
};
