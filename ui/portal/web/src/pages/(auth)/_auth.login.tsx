import { createFileRoute } from "@tanstack/react-router";

import { WizardProvider } from "@left-curve/applets-kit";
import { deserializeJson } from "@left-curve/dango/encoding";
import { Login } from "~/components/auth/Login";

export const Route = createFileRoute("/(auth)/_auth/login")({
  loader: () => {
    const isFirstVisit = localStorage.getItem("dango.firstVisit");
    return {
      isFirstVisit: !isFirstVisit ? true : deserializeJson<boolean>(isFirstVisit),
    };
  },
  component: LoginComponent,
});

function LoginComponent() {
  const { isFirstVisit } = Route.useLoaderData();
  return (
    <WizardProvider wrapper={<Login isFirstVisit={isFirstVisit} />}>
      <Login.Username />
      <Login.Credential />
    </WizardProvider>
  );
}
