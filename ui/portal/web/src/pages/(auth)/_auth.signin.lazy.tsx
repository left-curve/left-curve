import { createLazyFileRoute, useSearch } from "@tanstack/react-router";

import { WizardProvider } from "@left-curve/applets-kit";
import { useEffect } from "react";
import { Signin } from "~/components/auth/Signin";
import { Modals } from "~/components/modals/RootModal";
import { useApp } from "~/hooks/useApp";

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
