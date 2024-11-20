"use client";

import { useAccount } from "@leftcurve/react";
import { useEffect } from "react";

import { ManageMargin } from "./ManageMargin";
import { ManageSafe } from "./ManageSafe";
import { ManageSpot } from "./ManageSpot";

import { AccountType } from "@leftcurve/types";

interface Props {
  index: number;
}

export const AccountRouter: React.FC<Props> = ({ index }) => {
  const { account, accounts, changeAccount } = useAccount();

  useEffect(() => {
    if (!account || !accounts?.length || account.index === index) return;
    const newAccount = accounts.find((a) => a.index === index);
    if (!newAccount) throw new Error(`Account with index ${index} not found`);
    changeAccount?.(newAccount);
  }, [index, account, accounts, changeAccount]);

  if (!account || account.index !== index) return null;

  switch (account.type) {
    case AccountType.Spot:
      return <ManageSpot />;
    case AccountType.Safe:
      return <ManageSafe />;
    case AccountType.Margin:
      return <ManageMargin />;
  }
};
