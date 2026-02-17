import { m } from "@left-curve/foundation/paraglide/messages.js";
import { View } from "react-native";

import { GlobalText } from "~/components/foundation";

type WarningTransferAccountsProps = {
  variant: "send" | "receive";
};

export const WarningTransferAccounts: React.FC<WarningTransferAccountsProps> = ({ variant }) => {
  const receiveWarnings = [
    m["transfer.warning.receiveCex"]({ app: "Dango" }),
    m["transfer.warning.deposit"](),
    m["transfer.warning.receivePreMessage"](),
  ];

  const sendWarnings = [
    m["transfer.warning.sendNonDango"]({ app: "Dango" }),
    m["transfer.warning.withdraw"](),
  ];

  const warnings = variant === "receive" ? receiveWarnings : sendWarnings;

  return (
    <View className="rounded-xl p-3 bg-surface-secondary-rice border border-outline-secondary-gray gap-2">
      {variant === "receive" ? (
        <GlobalText className="diatype-sm-bold">{m["transfer.warning.receiveTitle"]()}</GlobalText>
      ) : null}
      {warnings.map((warning) => (
        <GlobalText className="diatype-sm-regular text-ink-tertiary-500" key={warning}>
          {`• ${warning}`}
        </GlobalText>
      ))}
    </View>
  );
};
