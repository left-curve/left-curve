import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { AnchorHTMLAttributes, PropsWithChildren } from "react";

import { WarningTransferAccounts } from "../src/components/transfer/WarningTransferAccounts";

vi.mock("@tanstack/react-router", async () => {
  const React = await import("react");

  const Link = React.forwardRef<
    HTMLAnchorElement,
    PropsWithChildren<AnchorHTMLAttributes<HTMLAnchorElement> & { to: string }>
  >(({ children, to, ...props }, ref) => (
    <a href={to} ref={ref} {...props}>
      {children}
    </a>
  ));

  Link.displayName = "TestRouterLink";

  return {
    Link,
  };
});

function expectInterpolatedWarning(message: string, appLabel: string) {
  const [prefix = "", suffix = ""] = message.split("{app}");
  const paragraph = screen.getByRole("link", { name: appLabel }).closest("p");

  expect(paragraph).toHaveTextContent(prefix.trim());
  expect(paragraph).toHaveTextContent(appLabel);
  expect(paragraph).toHaveTextContent(suffix.trim());
}

describe("transfer account warnings", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders the send warning with a bridge withdrawal link", () => {
    const appLabel = m["transfer.warning.withdraw"]();

    render(<WarningTransferAccounts variant="send" />);

    expectInterpolatedWarning(
      m["transfer.warning.sendNonDango"]({
        app: "{app}",
      }),
      appLabel,
    );
    expect(screen.getByRole("link", { name: appLabel })).toHaveAttribute("href", "/bridge");
  });

  it("renders the receive warning title, pre-message, and bridge deposit link", () => {
    const appLabel = m["transfer.warning.deposit"]();

    render(<WarningTransferAccounts variant="receive" />);

    expect(screen.getByText(m["transfer.warning.receiveTitle"]())).toBeInTheDocument();
    expect(screen.getByText(m["transfer.warning.receivePreMessage"]())).toBeInTheDocument();
    expectInterpolatedWarning(
      m["transfer.warning.receiveCex"]({
        app: "{app}",
      }),
      appLabel,
    );
    expect(screen.getByRole("link", { name: appLabel })).toHaveAttribute("href", "/bridge");
  });

  it("does not render when hidden", () => {
    const { container } = render(<WarningTransferAccounts isVisible={false} variant="send" />);

    expect(container).toBeEmptyDOMElement();
  });
});
