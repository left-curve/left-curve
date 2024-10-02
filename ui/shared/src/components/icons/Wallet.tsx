import { type ConnectorId, ConnectorIdType } from "@leftcurve/types";
import { KeplrIcon } from "./wallets/Keplr";
import { MetamaskIcon } from "./wallets/Metamask";
import { PasskeyIcon } from "./wallets/Passkey";

interface Props extends React.SVGAttributes<HTMLOrSVGElement> {
  connectorId: ConnectorId;
}

export const WalletIcon: React.FC<Props> = ({ connectorId, ...props }) => {
  switch (connectorId) {
    case ConnectorIdType.Metamask:
      return <MetamaskIcon {...props} />;
    case ConnectorIdType.Keplr:
      return <KeplrIcon {...props} />;
    case ConnectorIdType.Passkey:
      return <PasskeyIcon {...props} />;
  }
};
