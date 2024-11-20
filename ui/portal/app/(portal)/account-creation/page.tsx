import { WizardProvider } from "@dango/shared";
import { SelectStep } from "./SelectStep";
import { Stepper } from "./Stepper";
import { TransferStep } from "./TransferStep";

export default function AccountCreationPage() {
  return (
    <WizardProvider wrapper={<Stepper />}>
      <SelectStep />
      <TransferStep />
    </WizardProvider>
  );
}
