import { createLazyFileRoute } from "@tanstack/react-router";

import { WizardProvider } from "@left-curve/applets-kit";
import { AccountCreation } from "~/components/account/AccountCreation";

export const Route = createLazyFileRoute("/(app)/_app/account/create")({
  component: AccountCreationApplet,
});

function AccountCreationApplet() {
  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-4 p-4 md:pt-28 items-center justify-start">
      <WizardProvider wrapper={<AccountCreation />}>
        <AccountCreation.TypeSelector />
        <AccountCreation.Deposit />
      </WizardProvider>
    </div>
  );
}
