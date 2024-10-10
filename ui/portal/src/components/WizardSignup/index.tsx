import { WizardProvider } from "@dango/shared";
import { ConnectStep } from "./ConnectStep";
import { CredentialStep } from "./CredentialStep";
import { WizardSignupWrapper } from "./Wrapper";

export const WizardSignup: React.FC = () => {
  return (
    <WizardProvider wrapper={<WizardSignupWrapper />}>
      <CredentialStep />
      <ConnectStep />
    </WizardProvider>
  );
};
