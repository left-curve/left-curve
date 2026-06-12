import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";

import { VaultAddLiquidity } from "../src/components/modals/VaultAddLiquidity";
import { VaultWithdrawLiquidity } from "../src/components/modals/VaultWithdrawLiquidity";
import { VaultWithdrawLiquidityWithPenalty } from "../src/components/modals/VaultWithdrawLiquidityWithPenalty";

const vaultModalMocks = vi.hoisted(() => ({
  hideModal: vi.fn(),
}));

type ModalRef = {
  triggerOnClose: () => void;
};

function getIconOnlyButton(container: HTMLElement) {
  const button = Array.from(container.querySelectorAll<HTMLButtonElement>("button")).find(
    (candidate) => candidate.textContent?.trim() === "",
  );
  if (!button) throw new Error("Expected an icon-only modal button to exist");
  return button;
}

describe("vault liquidity modals", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: vaultModalMocks.hideModal,
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
    });
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it("confirms an add-liquidity request and keeps the close rejection path available", () => {
    const confirmAddLiquidity = vi.fn();
    const rejectAddLiquidity = vi.fn();
    const modalRef = createRef<ModalRef>();

    render(
      <VaultAddLiquidity
        ref={modalRef}
        amount="1234.56"
        confirmAddLiquidity={confirmAddLiquidity}
        rejectAddLiquidity={rejectAddLiquidity}
        sharesToReceive="12.34"
      />,
    );

    expect(screen.getByText(m["vaultLiquidity.modal.addLiquidity"]())).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.modal.depositing"]())).toBeInTheDocument();
    expect(screen.getByText("$1,234.56")).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.modal.destination"]())).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.title"]())).toBeInTheDocument();
    expect(screen.getByText("$0.02")).toBeInTheDocument();

    modalRef.current?.triggerOnClose();
    expect(rejectAddLiquidity).toHaveBeenCalledOnce();
    expect(vaultModalMocks.hideModal).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(confirmAddLiquidity).toHaveBeenCalledOnce();
    expect(vaultModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("preserves zero-valued backend add-liquidity amounts in confirmation details", () => {
    const confirmAddLiquidity = vi.fn();
    const rejectAddLiquidity = vi.fn();

    render(
      <VaultAddLiquidity
        amount="0"
        confirmAddLiquidity={confirmAddLiquidity}
        rejectAddLiquidity={rejectAddLiquidity}
        sharesToReceive="0"
      />,
    );

    expect(screen.getByText(m["vaultLiquidity.modal.depositing"]())).toBeInTheDocument();
    expect(screen.getByText("$0.00")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(confirmAddLiquidity).toHaveBeenCalledOnce();
    expect(rejectAddLiquidity).not.toHaveBeenCalled();
    expect(vaultModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("rejects add-liquidity from the visible close control without confirming", () => {
    const confirmAddLiquidity = vi.fn();
    const rejectAddLiquidity = vi.fn();

    const { container } = render(
      <VaultAddLiquidity
        amount="1234.56"
        confirmAddLiquidity={confirmAddLiquidity}
        rejectAddLiquidity={rejectAddLiquidity}
        sharesToReceive="12.34"
      />,
    );

    fireEvent.click(getIconOnlyButton(container));

    expect(rejectAddLiquidity).toHaveBeenCalledOnce();
    expect(confirmAddLiquidity).not.toHaveBeenCalled();
    expect(vaultModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("confirms a normal withdrawal and shows the cooldown warning", () => {
    const confirmWithdrawal = vi.fn();
    const rejectWithdrawal = vi.fn();
    const modalRef = createRef<ModalRef>();

    render(
      <VaultWithdrawLiquidity
        ref={modalRef}
        confirmWithdrawal={confirmWithdrawal}
        rejectWithdrawal={rejectWithdrawal}
        sharesToBurn="12.34"
        usdToReceive="1234.56"
      />,
    );

    expect(screen.getByText(m["vaultLiquidity.modal.withdrawLiquidity"]())).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.modal.withdrawing"]())).toBeInTheDocument();
    expect(screen.getByText("$1,234.56")).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.modal.from"]())).toBeInTheDocument();
    expect(screen.getByText("$0.00")).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.modal.cooldownTitle"]())).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.modal.cooldownDescription"]())).toBeInTheDocument();

    modalRef.current?.triggerOnClose();
    expect(rejectWithdrawal).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(confirmWithdrawal).toHaveBeenCalledOnce();
    expect(vaultModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("preserves zero-valued backend withdrawal amounts in confirmation details", () => {
    const confirmWithdrawal = vi.fn();
    const rejectWithdrawal = vi.fn();

    render(
      <VaultWithdrawLiquidity
        confirmWithdrawal={confirmWithdrawal}
        rejectWithdrawal={rejectWithdrawal}
        sharesToBurn="0"
        usdToReceive="0"
      />,
    );

    expect(screen.getByText(m["vaultLiquidity.modal.withdrawing"]())).toBeInTheDocument();
    expect(screen.getAllByText("$0.00")).toHaveLength(2);

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(confirmWithdrawal).toHaveBeenCalledOnce();
    expect(rejectWithdrawal).not.toHaveBeenCalled();
    expect(vaultModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("rejects a normal withdrawal from the visible close control without confirming", () => {
    const confirmWithdrawal = vi.fn();
    const rejectWithdrawal = vi.fn();

    const { container } = render(
      <VaultWithdrawLiquidity
        confirmWithdrawal={confirmWithdrawal}
        rejectWithdrawal={rejectWithdrawal}
        sharesToBurn="12.34"
        usdToReceive="1234.56"
      />,
    );

    fireEvent.click(getIconOnlyButton(container));

    expect(rejectWithdrawal).toHaveBeenCalledOnce();
    expect(confirmWithdrawal).not.toHaveBeenCalled();
    expect(vaultModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("shows early-withdrawal penalty math and supports cancel or withdraw actions", () => {
    const confirmWithdrawal = vi.fn();
    const rejectWithdrawal = vi.fn();

    render(
      <VaultWithdrawLiquidityWithPenalty
        confirmWithdrawal={confirmWithdrawal}
        penaltyEndDate="2026-08-15"
        penaltyPercentage={12.5}
        rejectWithdrawal={rejectWithdrawal}
        usdToWithdraw="1000"
      />,
    );

    expect(screen.getByText(m["vaultLiquidity.modal.earlyWithdrawalTitle"]())).toBeInTheDocument();
    expect(
      screen.getByText(
        m["vaultLiquidity.modal.earlyWithdrawalDescription"]({ date: "2026-08-15" }),
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("$1,000.00")).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.modal.penalty"]())).toBeInTheDocument();
    expect(screen.getByText("(12.5%)")).toBeInTheDocument();
    expect(screen.getByText("-$125.00")).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.modal.youReceive"]())).toBeInTheDocument();
    expect(screen.getByText("$875.00")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["common.cancel"]() }));

    expect(rejectWithdrawal).toHaveBeenCalledOnce();
    expect(confirmWithdrawal).not.toHaveBeenCalled();
    expect(vaultModalMocks.hideModal).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: m["common.withdraw"]() }));

    expect(confirmWithdrawal).toHaveBeenCalledOnce();
    expect(vaultModalMocks.hideModal).toHaveBeenCalledTimes(2);
  });

  it("rejects an early withdrawal from the visible close control without confirming", () => {
    const confirmWithdrawal = vi.fn();
    const rejectWithdrawal = vi.fn();

    const { container } = render(
      <VaultWithdrawLiquidityWithPenalty
        confirmWithdrawal={confirmWithdrawal}
        penaltyEndDate="2026-08-15"
        penaltyPercentage={12.5}
        rejectWithdrawal={rejectWithdrawal}
        usdToWithdraw="1000"
      />,
    );

    fireEvent.click(getIconOnlyButton(container));

    expect(rejectWithdrawal).toHaveBeenCalledOnce();
    expect(confirmWithdrawal).not.toHaveBeenCalled();
    expect(vaultModalMocks.hideModal).toHaveBeenCalledOnce();
  });
});
