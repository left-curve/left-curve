import { createFileRoute } from "@tanstack/react-router";

import { WizardProvider } from "@left-curve/applets-kit";
import {
  CreateAccountDepositStep,
  CreateAccountTypeStep,
  CreateAccountWrapper,
} from "~/components/create-account";

export const Route = createFileRoute("/(app)/_app/create-account")({
  component: CreateAccountComponent,
});

function CreateAccountComponent() {
  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-4 p-4 md:pt-28 items-center justify-start">
      <WizardProvider wrapper={<CreateAccountWrapper />}>
        <CreateAccountTypeStep />
        <CreateAccountDepositStep />
      </WizardProvider>
    </div>
  );
}
