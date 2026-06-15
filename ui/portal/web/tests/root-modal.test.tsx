import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import * as React from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseAppFactory,
  setAppletsKitUseMediaQueryFactory,
} from "./mocks/applets-kit";

import { Modals } from "@left-curve/applets-kit";

const rootModalMocks = vi.hoisted(() => ({
  hideModal: vi.fn(),
  isMd: true,
  modal: {
    modal: undefined as string | undefined,
    props: {} as Record<string, unknown>,
  },
  triggerOnClose: vi.fn(),
}));

let RootModal: typeof import("../src/components/modals/RootModal").RootModal;

function createMockModal(label: string) {
  const renderSpy = vi.fn();
  const Component = React.forwardRef(
    (props: Record<string, unknown>, ref: React.ForwardedRef<unknown>) => {
      renderSpy(props, ref);
      React.useImperativeHandle(ref, () => ({
        triggerOnClose: rootModalMocks.triggerOnClose,
      }));

      return <div>{label}</div>;
    },
  );

  return { Component, renderSpy };
}

const confirmSendModal = createMockModal("confirm-send-modal");
const renewSessionModal = createMockModal("renew-session-modal");
const signWithDesktopModal = createMockModal("sign-with-desktop-modal");

function findLazyModalText(text: string) {
  return screen.findByText(text, undefined, { timeout: 10000 });
}

vi.mock("framer-motion", () => ({
  AnimatePresence: ({ children }: React.PropsWithChildren) => <>{children}</>,
  motion: {
    div: ({
      animate: _animate,
      children,
      exit: _exit,
      initial: _initial,
      ...props
    }: React.HTMLAttributes<HTMLDivElement> & {
      animate?: unknown;
      exit?: unknown;
      initial?: unknown;
    }) => <div {...props}>{children}</div>,
  },
}));

vi.mock("react-error-boundary", () => ({
  ErrorBoundary: ({ children }: React.PropsWithChildren) => <>{children}</>,
}));

vi.mock("react-modal-sheet", () => {
  const Sheet = ({
    children,
    disableDrag,
    isOpen,
    onClose,
  }: React.PropsWithChildren<{
    disableDrag?: boolean;
    isOpen: boolean;
    onClose: () => void;
  }>) => (
    <section data-disable-drag={String(disableDrag)} data-open={String(isOpen)} data-testid="sheet">
      <button onClick={onClose} type="button">
        sheet-close
      </button>
      {children}
    </section>
  );

  Sheet.Container = ({ children, className }: React.PropsWithChildren<{ className?: string }>) => (
    <div className={className} data-testid="sheet-container">
      {children}
    </div>
  );
  Sheet.Header = ({ children }: React.PropsWithChildren) => <header>{children}</header>;
  Sheet.Content = ({ children }: React.PropsWithChildren) => <main>{children}</main>;
  Sheet.Scroller = ({ children }: React.PropsWithChildren) => <div>{children}</div>;
  Sheet.Backdrop = ({ onTap }: { onTap: () => void }) => (
    <button onClick={onTap} type="button">
      sheet-backdrop
    </button>
  );

  return { Sheet };
});

vi.mock("../src/components/foundation/ChunkErrorFallback", () => ({
  ChunkErrorFallback: () => <div>chunk-error</div>,
}));

vi.mock("../src/components/modals/ConfirmSend", () => ({
  ConfirmSend: confirmSendModal.Component,
}));

vi.mock("../src/components/modals/RenewSession", () => ({
  RenewSession: renewSessionModal.Component,
}));

vi.mock("../src/components/modals/SignWithDesktop", () => ({
  SignWithDesktop: signWithDesktopModal.Component,
}));

describe("root modal", () => {
  beforeAll(async () => {
    ({ RootModal } = await import("../src/components/modals/RootModal"));
  });

  beforeEach(() => {
    resetAppletsKitMocks();
    rootModalMocks.isMd = true;
    rootModalMocks.modal.modal = undefined;
    rootModalMocks.modal.props = {};
    setAppletsKitUseAppFactory(() => ({
      hideModal: rootModalMocks.hideModal,
      modal: rootModalMocks.modal,
    }));
    setAppletsKitUseMediaQueryFactory(() => ({
      isMd: rootModalMocks.isMd,
    }));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders nothing when no modal is active", () => {
    const { container } = render(<RootModal />);

    expect(container).toBeEmptyDOMElement();
  });

  it("renders desktop modals with props and calls triggerOnClose when the overlay closes", async () => {
    rootModalMocks.modal.modal = Modals.ConfirmSend;
    rootModalMocks.modal.props = {
      amount: "1000000",
      to: "0x726563697069656e740000000000000000000000",
    };

    render(<RootModal />);

    expect(await findLazyModalText("confirm-send-modal")).toBeInTheDocument();
    await waitFor(() => {
      expect(confirmSendModal.renderSpy).toHaveBeenCalledWith(
        expect.objectContaining(rootModalMocks.modal.props),
        expect.anything(),
      );
    });

    fireEvent.click(screen.getByText("confirm-send-modal").parentElement!);

    expect(rootModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(rootModalMocks.triggerOnClose).toHaveBeenCalledOnce();
  });

  it("does not close protected desktop modals from the overlay", async () => {
    rootModalMocks.modal.modal = Modals.RenewSession;

    render(<RootModal />);

    expect(await findLazyModalText("renew-session-modal")).toBeInTheDocument();

    fireEvent.click(screen.getByText("renew-session-modal").parentElement!);

    expect(rootModalMocks.hideModal).not.toHaveBeenCalled();
    expect(rootModalMocks.triggerOnClose).not.toHaveBeenCalled();
  });

  it("renders mobile sheets with headers and closes through sheet controls", async () => {
    rootModalMocks.isMd = false;
    rootModalMocks.modal.modal = Modals.SignWithDesktop;

    render(<RootModal />);

    expect(await findLazyModalText("sign-with-desktop-modal")).toBeInTheDocument();
    expect(screen.getByTestId("sheet")).toHaveAttribute("data-disable-drag", "undefined");
    expect(screen.getByText(m["common.signin"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "sheet-close" }));

    expect(rootModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(rootModalMocks.triggerOnClose).toHaveBeenCalledOnce();
  });

  it("prevents mobile backdrop closing for disableClosing modals", async () => {
    rootModalMocks.isMd = false;
    rootModalMocks.modal.modal = Modals.RenewSession;

    render(<RootModal />);

    expect(await findLazyModalText("renew-session-modal")).toBeInTheDocument();
    expect(screen.getByTestId("sheet")).toHaveAttribute("data-disable-drag", "true");

    fireEvent.click(screen.getByRole("button", { name: "sheet-backdrop" }));

    expect(rootModalMocks.hideModal).not.toHaveBeenCalled();
    expect(rootModalMocks.triggerOnClose).not.toHaveBeenCalled();
  });

  it("loads the activity conversion modal from the registry and closes from its control", async () => {
    rootModalMocks.modal.modal = Modals.ActivityConvert;

    const { container } = render(<RootModal />);

    expect(await findLazyModalText(m["activities.activity.modal.swapped"]())).toBeInTheDocument();
    expect(screen.getByText(m["activities.activity.modal.fee"]())).toBeInTheDocument();
    expect(screen.getByText(m["activities.activity.modal.time"]())).toBeInTheDocument();
    expect(screen.getByText(m["activities.activity.modal.transaction"]())).toBeInTheDocument();

    const closeButton = container.querySelector("button.absolute");
    if (!closeButton) throw new Error("Expected activity conversion close button");

    fireEvent.click(closeButton);

    expect(rootModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(rootModalMocks.triggerOnClose).not.toHaveBeenCalled();
  });
});
