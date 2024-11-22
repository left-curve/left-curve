import { useAccountInfo } from "@leftcurve/react";

import { Spinner } from "@dango/shared";
import { ManageMargin } from "./ManageMargin";
import { ManageSafe } from "./ManageSafe";
import { ManageSpot } from "./ManageSpot";

import { AccountType, type Address } from "@leftcurve/types";
import { Navigate } from "react-router-dom";

interface Props {
  address: Address;
}

export const AccountRouter: React.FC<Props> = ({ address }) => {
  const { isLoading, data: account } = useAccountInfo({ address, query: { retry: 0 } });

  if (isLoading) return <Spinner />;

  if (!account) return <Navigate to="/404" />;

  switch (account.type) {
    case AccountType.Spot:
      return <ManageSpot account={account} />;
    case AccountType.Safe:
      return <ManageSafe account={account} />;
    case AccountType.Margin:
      return <ManageMargin account={account} />;
  }
};
