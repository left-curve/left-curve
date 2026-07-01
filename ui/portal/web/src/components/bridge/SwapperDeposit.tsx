import { useAccount, useBalances } from "@left-curve/store";

import { Button, IconChevronRight, WarningContainer, useTheme } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  SwapperIframe,
  WidgetEventName,
  type ComponentStyles,
  type SwapperStyles,
} from "@swapper-finance/deposit-sdk";

import { useEffect, useRef } from "react";
import { DepositFeeBadge } from "./DepositOptions";

const SWAPPER_DST_CHAIN_ID = "dango";
const SWAPPER_DST_TOKEN_ADDR = "usdc";
const SWAPPER_IFRAME_HEIGHT = "560px";
const SWAPPER_DEPOSIT_OPTIONS = ["transferCrypto", "depositWithCash", "walletDeposit"] as const;
const SWAPPER_BASE_COMPONENT_STYLES = {
  width: "100%",
  primaryColor: "#F57589",
  accentColor: "#F57589",
  sphereColor: "#F57589",
  border: "0",
  borderRadius: "12px",
} satisfies ComponentStyles;

const getCssVariableColor = (variable: string) => {
  const color = getComputedStyle(document.documentElement).getPropertyValue(variable).trim();
  return color || `var(${variable})`;
};

const getSwapperStyles = (theme: "light" | "dark"): SwapperStyles => ({
  themeMode: theme,
  componentStyles: {
    ...SWAPPER_BASE_COMPONENT_STYLES,
    backgroundColor: getCssVariableColor("--color-surface-secondary-rice"),
    primaryButtonTextColor: getCssVariableColor("--color-surface-primary-rice"),
    surfaceColor: getCssVariableColor("--color-surface-tertiary-rice"),
    surfaceAltColor: getCssVariableColor("--color-surface-quaternary-rice"),
    textColor: getCssVariableColor("--color-ink-primary-900"),
  },
});

type SwapperDepositProps = {
  onBack: () => void;
};

export const SwapperDeposit = ({ onBack }: SwapperDepositProps) => {
  const { account, isConnected, refreshUserStatus } = useAccount();
  const { refetch: refreshBalances } = useBalances({ address: account?.address });
  const { theme } = useTheme();
  const containerRef = useRef<HTMLDivElement>(null);
  const swapperIntegratorId = import.meta.env.PUBLIC_SWAPPER_INTEGRATOR_ID?.trim();

  useEffect(() => {
    const container = containerRef.current;
    const depositWalletAddress = account?.address;

    if (!container || !isConnected || !depositWalletAddress || !swapperIntegratorId) return;

    container.replaceChildren();

    const swapper = new SwapperIframe({
      container,
      integratorId: swapperIntegratorId,
      dstChainId: SWAPPER_DST_CHAIN_ID,
      dstTokenAddr: SWAPPER_DST_TOKEN_ADDR,
      depositWalletAddress,
      supportedDepositOptions: [...SWAPPER_DEPOSIT_OPTIONS],
      styles: getSwapperStyles(theme === "dark" ? "dark" : "light"),
      iframeAttributes: {
        width: "100%",
        minWidth: "0",
        height: SWAPPER_IFRAME_HEIGHT,
        borderRadius: "12px",
        allowtransparency: "true",
      },
      onEvent: (event) => {
        if (event.type !== WidgetEventName.TRANSACTION_SUCCESS) return;
        refreshBalances();
        refreshUserStatus?.();
      },
    });

    const iframe = swapper.getIframe();
    iframe.style.backgroundColor = "transparent";
    iframe.style.display = "block";
    iframe.setAttribute("allowtransparency", "true");
    iframe.parentElement?.style.setProperty("height", "fit-content");

    return () => {
      swapper.destroy();
      container.replaceChildren();
    };
  }, [
    account?.address,
    isConnected,
    refreshBalances,
    refreshUserStatus,
    swapperIntegratorId,
    theme,
  ]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between gap-3">
        <Button variant="link" size="sm" className="m-0 p-0" onClick={onBack}>
          <IconChevronRight className="h-4 w-4 rotate-180" />
          {m["bridge.deposit.moreOptions.back"]()}
        </Button>
        <DepositFeeBadge />
      </div>

      {!account?.address || !isConnected || !swapperIntegratorId ? (
        <WarningContainer color="error" description={m["common.failedToLoad"]()} />
      ) : (
        <div
          ref={containerRef}
          className="h-fit w-full overflow-hidden rounded-xl bg-surface-secondary-rice shadow-account-card"
        />
      )}
    </div>
  );
};
