import { WizardProvider } from "../../../../packages/ui/build/index.mjs";
import type React from "react";
import { SelectStep } from "./SelectStep";
import { Stepper } from "./Stepper";
import { TransferStep } from "./TransferStep";

export const AccountCreation: React.FC = () => {
  return (
    <WizardProvider wrapper={<Stepper />}>
      <SelectStep />
      <TransferStep />
    </WizardProvider>
  );
};
