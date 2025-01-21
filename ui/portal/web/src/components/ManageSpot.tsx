import { AccountInfo, SpotEditAccount, SpotPortfolioTable } from "@left-curve/portal-shared";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import type { Account } from "@left-curve/types";

interface Props {
  account: Account;
}

const ManageSpot: React.FC<Props> = ({ account }) => {
  const navigate = useNavigate();
  const [isEditing, setIsEditing] = useState(false);

  return (
    <div className="flex flex-1 flex-col w-full items-center gap-14 justify-center">
      {isEditing ? (
        <SpotEditAccount goBack={() => setIsEditing(false)} />
      ) : (
        <>
          <AccountInfo
            account={account}
            avatarUri="/images/avatars/spot.svg"
            triggerEdit={() => setIsEditing(true)}
          />
          <SpotPortfolioTable
            account={account}
            sendAction={() => navigate({ to: "/transfer?action=send" })}
            receiveAction={() => navigate({ to: "/transfer?action=receive" })}
          />
        </>
      )}
    </div>
  );
};

export default ManageSpot;
