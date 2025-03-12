import { BackpackIcon } from "./wallets/Backpack";
import { EthereumIcon } from "./wallets/Ethereum";
import { KeplrIcon } from "./wallets/Keplr";
import { PhantomIcon } from "./wallets/Phantom";

interface Props extends React.SVGAttributes<HTMLOrSVGElement> {
  connectorId: string;
}

export const WalletIcon: React.FC<Props> = ({ connectorId, ...props }) => {
  switch (connectorId) {
    case "metamask":
      return <EthereumIcon {...props} />;
    case "keplr":
      return <KeplrIcon {...props} />;
    case "phantom":
      return <PhantomIcon {...props} />;
    case "backpack":
      return <BackpackIcon {...props} />;
    default:
      return <EthereumIcon {...props} />;
  }
};
