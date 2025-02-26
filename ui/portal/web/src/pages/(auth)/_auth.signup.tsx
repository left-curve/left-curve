import { WizardProvider } from "@left-curve/applets-kit";
import { createFileRoute } from "@tanstack/react-router";

import { SignupCredentialStep, SignupUsernameStep, SignupWrapper } from "~/components/signup";

export const Route = createFileRoute("/(auth)/_auth/signup")({
  component: SignupComponent,
});

function SignupComponent() {
  return (
    <WizardProvider wrapper={<SignupWrapper />} persistKey="signup-form">
      <SignupCredentialStep />
      <SignupUsernameStep />
    </WizardProvider>
  );
}
