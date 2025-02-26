import { createFileRoute } from "@tanstack/react-router";

import { WizardProvider } from "@left-curve/applets-kit";
import { LoginCredentialStep, LoginUsernameStep, LoginWrapper } from "~/components/login";

export const Route = createFileRoute("/(auth)/_auth/login")({
  component: LoginComponent,
});

function LoginComponent() {
  return (
    <WizardProvider wrapper={<LoginWrapper />}>
      <LoginUsernameStep />
      <LoginCredentialStep />
    </WizardProvider>
  );
}
