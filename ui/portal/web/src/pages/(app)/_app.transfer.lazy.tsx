import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { MobileTitle } from "~/components/foundation/MobileTitle";
import { Transfer } from "~/components/transfer/Transfer";

export const Route = createLazyFileRoute("/(app)/_app/transfer")({
  component: TransferApplet,
});

function TransferApplet() {
  const { action } = Route.useSearch();
  const navigate = useNavigate({ from: "/transfer" });
  const changeAction = (action: string) => navigate({ search: { action }, replace: true });

  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
      <MobileTitle title={m["sendAndReceive.title"]()} />
      <div className="w-full flex flex-col gap-4 md:pt-28 items-center justify-start">
        <Transfer action={action} changeAction={changeAction}>
          <Transfer.Send />
          <Transfer.SpotPerp />
        </Transfer>
      </div>
    </div>
  );
}
