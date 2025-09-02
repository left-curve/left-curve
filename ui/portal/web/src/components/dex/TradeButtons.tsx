import {
  Button,
  IconButton,
  IconChevronRight,
  twMerge,
  useApp,
  usePortalTarget,
} from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { createPortal } from "react-dom";

import { m } from "~/paraglide/messages";

import type { useProTradeState } from "@left-curve/store";
import type React from "react";

type TradeButtonsProps = {
  state: ReturnType<typeof useProTradeState>;
};

export const TradeButtons: React.FC<TradeButtonsProps> = ({ state }) => {
  const navigate = useNavigate();
  const { setTradeBarVisibility } = useApp();
  const { isConnected } = useAccount();

  const { changeAction } = state;

  const container = usePortalTarget("#trade-buttons");

  return (
    <>
      {container
        ? createPortal(
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
                <div className="flex-1 flex gap-2 ">
                  <Button
                    className="h-[44px]"
                    fullWidth
                    variant="tertiary"
                    onClick={() => {
                      setTradeBarVisibility(true);
                      changeAction("buy");
                    }}
                  >
                    {m["proSwap.buy"]()}
                  </Button>
                  <Button
                    className="h-[44px]"
                    fullWidth
                    onClick={() => {
                      setTradeBarVisibility(true);
                      changeAction("sell");
                    }}
                  >
                    {m["proSwap.sell"]()}
                  </Button>
                </div>
              ) : (
                <Button className="flex-1 h-full" onClick={() => navigate({ to: "/signin" })}>
                  Connect
                </Button>
              )}
            </div>,
            container,
          )
        : null}
    </>
  );
};
