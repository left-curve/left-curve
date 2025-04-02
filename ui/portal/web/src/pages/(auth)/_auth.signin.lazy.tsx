import { createLazyFileRoute, useSearch } from "@tanstack/react-router";

import { WizardProvider } from "@left-curve/applets-kit";
import { Signin } from "~/components/auth/Signin";

export const Route = createLazyFileRoute("/(auth)/_auth/signin")({
  component: SigninComponent,
});

function SigninComponent() {
  const { socketId } = useSearch({ strict: false });

  return (
    <WizardProvider wrapper={<Signin />} defaultData={{ socketId }} startIndex={socketId ? 2 : 0}>
      <Signin.Username />
      <Signin.Credential />
      <Signin.Mobile />
    </WizardProvider>
  );
}
