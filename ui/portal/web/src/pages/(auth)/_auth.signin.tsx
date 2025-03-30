import { createFileRoute } from "@tanstack/react-router";

import { WizardProvider } from "@left-curve/applets-kit";
import { deserializeJson } from "@left-curve/dango/encoding";
import { Signin } from "~/components/auth/Signin";

export const Route = createFileRoute("/(auth)/_auth/signin")({
  loader: () => {
    const isFirstVisit = localStorage.getItem("dango.firstVisit");
    return {
      isFirstVisit: !isFirstVisit ? true : deserializeJson<boolean>(isFirstVisit),
    };
  },
  component: SigninComponent,
});

function SigninComponent() {
  const { isFirstVisit } = Route.useLoaderData();
  return (
    <WizardProvider wrapper={<Signin isFirstVisit={isFirstVisit} />}>
      <Signin.Username />
      <Signin.Credential />
    </WizardProvider>
  );
}
