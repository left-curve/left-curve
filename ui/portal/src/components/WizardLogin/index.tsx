import { WizardProvider } from "@dango/shared";
import { ConnectorStep } from "./ConnectorStep";
import { LoginStep } from "./LoginStep";
import { WizardLoginWrapper } from "./Wrapper";

export const WizardLogin: React.FC = () => {
  return (
    <WizardProvider wrapper={<WizardLoginWrapper />}>
      <LoginStep />
      <ConnectorStep />
    </WizardProvider>
  );
};
