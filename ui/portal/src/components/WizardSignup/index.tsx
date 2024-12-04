import { ConnectStep } from "./ConnectStep";
import { CredentialStep } from "./CredentialStep";
import { TransferStep } from "./TransferStep";

export const WizardSignup: React.FC = () => {
  return (
    <>
      <CredentialStep />
      <ConnectStep />
      <TransferStep />
    </>
  );
};
