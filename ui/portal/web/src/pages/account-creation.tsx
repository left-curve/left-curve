import { createLazyRoute } from "@tanstack/react-router";
import { AccountCreation } from "~/components/AccountCreation";

export const AccountCreationRoute = createLazyRoute("/account-creation")({
  component: AccountCreation,
});
