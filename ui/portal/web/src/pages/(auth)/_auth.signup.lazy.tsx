import { WizardProvider } from "@left-curve/applets-kit";
import { createLazyFileRoute } from "@tanstack/react-router";
import { Signup } from "~/components/auth/Signup";

export const Route = createLazyFileRoute("/(auth)/_auth/signup")({
  component: SignupComponent,
});

function SignupComponent() {
  return (
    <WizardProvider wrapper={<Signup />} persistKey="dango.signup">
      <Signup.Credential />
      <Signup.Username />
      <Signup.Signin />
    </WizardProvider>
  );
}
