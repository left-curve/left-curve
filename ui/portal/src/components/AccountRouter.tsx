import { lazy } from "react";

import { type Account, AccountType } from "@left-curve/types";

const ManageSpot = lazy(() => import("./ManageSpot"));
const ManageSafe = lazy(() => import("./ManageSafe"));
const ManageMargin = lazy(() => import("./ManageMargin"));

interface Props {
  account: Account;
}

export const AccountRouter: React.FC<Props> = ({ account }) => {
  switch (account.type) {
    case AccountType.Spot:
      return <ManageSpot account={account} />;
    case AccountType.Safe:
      return <ManageSafe account={account} />;
    case AccountType.Margin:
      return <ManageMargin account={account} />;
  }
};
