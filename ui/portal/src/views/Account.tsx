import { isValidAddress } from "@left-curve/sdk";
import { useQueryState } from "nuqs";
import { useEffect } from "react";
import { redirect } from "react-router-dom";

import { AccountRouter } from "~/components/AccountRouter";

import type { Address } from "@left-curve/types";

const AccountView: React.FC = () => {
  const [address] = useQueryState("address");

  useEffect(() => {
    if (!address || !isValidAddress(address)) redirect("/404");
  }, [address]);

  if (!address) return null;

  return (
    <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
      <div className="flex flex-1 flex-col items-center justify-center gap-4 w-full">
        <AccountRouter address={address as Address} />
      </div>
    </div>
  );
};

export default AccountView;
