import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";
import { VaultLiquidity } from "~/components/earn/VaultLiquidity";

export const Route = createLazyFileRoute("/(app)/_app/earn/")({
  component: EarnApplet,
});

function EarnApplet() {
  const { action } = Route.useSearch();
  const navigate = useNavigate();

  const onChangeAction = (action: "deposit" | "withdraw") => {
    navigate({
      to: "/earn",
      replace: true,
      search: { action },
    });
  };

  return (
    <VaultLiquidity action={action} onChangeAction={onChangeAction}>
      <VaultLiquidity.Content />
    </VaultLiquidity>
  );
}
