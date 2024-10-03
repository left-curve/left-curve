"use client";

import { WizardProvider } from "@dango/shared";
import { ConnectStep } from "./ConnectStep";
import { CredentialStep } from "./CredentialStep";
import { WizardLoginWrapper } from "./Wrapper";

export const WizardLogin: React.FC = () => {
  return (
    <WizardProvider wrapper={<WizardLoginWrapper />}>
      <CredentialStep />
      <ConnectStep />
    </WizardProvider>
  );
};
