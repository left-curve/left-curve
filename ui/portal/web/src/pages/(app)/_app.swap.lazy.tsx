import { IconButton, IconChevronDown } from "@left-curve/applets-kit";
import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";

import { m } from "~/paraglide/messages";

import { SimpleSwap } from "~/components/dex/SimpleSwap";

export const Route = createLazyFileRoute("/(app)/_app/swap")({
  component: SwapApplet,
});

function SwapApplet() {
  const navigate = useNavigate();
  const { from, to } = Route.useSearch();

  const onChangePair = (pair: { from: string; to: string }) => {
    navigate({ to: ".", search: pair, replace: false });
  };

  return (
    <div className="w-full md:max-w-[25rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
      <h2 className="flex gap-2 items-center lg:hidden" onClick={() => navigate({ to: "/" })}>
        <IconButton variant="link">
          <IconChevronDown className="rotate-90" />
        </IconButton>

        <span className="h3-bold text-gray-900">{m["convert.title"]()}</span>
      </h2>
      <SimpleSwap pair={{ from, to }} onChangePair={onChangePair}>
        <SimpleSwap.Header />
        <SimpleSwap.Form />
        <SimpleSwap.Details />
        <SimpleSwap.Trigger />
      </SimpleSwap>
    </div>
  );
}
