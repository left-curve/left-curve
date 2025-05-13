import { WizardProvider } from "@left-curve/applets-kit";
import { createFileRoute } from "@tanstack/react-router";
import { ForgotUsername } from "~/components/auth/ForgotUsername";

export const Route = createFileRoute("/(auth)/_auth/forgot-username")({
  component: ForgotUsernameApplet,
});

function ForgotUsernameApplet() {
  return (
    <WizardProvider wrapper={<ForgotUsername />}>
      <ForgotUsername.Credential />
      <ForgotUsername.AvailableUsernames />
    </WizardProvider>
  );
}
