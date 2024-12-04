import { ConnectorStep } from "./ConnectorStep";
import { LoginStep } from "./LoginStep";

export const WizardLogin: React.FC = () => {
  return (
    <>
      <LoginStep />
      <ConnectorStep />
    </>
  );
};
