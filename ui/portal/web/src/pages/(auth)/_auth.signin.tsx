import { createFileRoute, useSearch } from "@tanstack/react-router";

import { WizardProvider } from "@left-curve/applets-kit";
import { deserializeJson } from "@left-curve/dango/encoding";
import { z } from "zod";
import { Signin } from "~/components/auth/Signin";

export const Route = createFileRoute("/(auth)/_auth/signin")({
  loader: () => {
    const isFirstVisit = localStorage.getItem("dango.firstVisit");
    return {
      isFirstVisit: !isFirstVisit ? true : deserializeJson<boolean>(isFirstVisit),
    };
  },
  component: SigninComponent,
  validateSearch: z.object({
    socketId: z.string().optional(),
  }),
});

function SigninComponent() {
  const { isFirstVisit } = Route.useLoaderData();
  const { socketId } = useSearch({ strict: false });

  return (
    <WizardProvider
      wrapper={<Signin isFirstVisit={socketId ? false : isFirstVisit} />}
      defaultData={{ socketId }}
      startIndex={socketId ? 2 : 0}
    >
      <Signin.Username />
      <Signin.Credential />
      <Signin.Mobile />
    </WizardProvider>
  );
}
