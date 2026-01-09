import { Link } from "@tanstack/react-router";
import { Button, WarningContainer } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

type WarningTransferAccountsProps = {
  variant: "send" | "receive";
  isVisible?: boolean;
};

const buildDescription = (message: string, appLabel: string) => {
  const [prefix, suffix] = message.split("{app}");

  return (
    <p>
      {prefix || message}
      <Button as={Link} to="/bridge" variant="link" size="xs" className="p-0 h-fit m-0 inline">
        {appLabel}
      </Button>
      {suffix}
    </p>
  );
};

const ReceiveDescription = () =>
  buildDescription(
    m["transfer.warning.receiveCex"]({ app: "{app}" }),
    m["transfer.warning.deposit"](),
  );

const SendDescription = () =>
  buildDescription(
    m["transfer.warning.sendNonDango"]({ app: "{app}" }),
    m["transfer.warning.withdraw"](),
  );

export const WarningTransferAccounts: React.FC<WarningTransferAccountsProps> = ({
  variant,
  isVisible = true,
}) => {
  if (!isVisible) return null;

  const description = variant === "receive" ? <ReceiveDescription /> : <SendDescription />;

  return <WarningContainer description={description} />;
};
