import { createLazyFileRoute, useSearch } from "@tanstack/react-router";

import { Modals, useApp, WizardProvider } from "@left-curve/applets-kit";
import { useEffect } from "react";
import { Signin } from "~/components/auth/Signin";

export const Route = createLazyFileRoute("/(auth)/_auth/signin")({
  component: SigninApplet,
});

function SigninApplet() {
  const { showModal } = useApp();
  const { socketId } = useSearch({ strict: false });

  useEffect(() => {
    if (socketId) showModal(Modals.SignWithDesktop, { socketId });
  }, []);

  return (
    <WizardProvider wrapper={<Signin />}>
      <Signin.Credential />
      <Signin.Username />
    </WizardProvider>
  );
}
