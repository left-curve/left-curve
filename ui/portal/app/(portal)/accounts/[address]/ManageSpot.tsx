"use client";

import { AccountInfo, SpotEditAccount, SpotPortfolioTable } from "@dango/shared";
import { useRouter } from "next/navigation";
import { useState } from "react";

export const ManageSpot: React.FC = () => {
  const { push: navigate } = useRouter();
  const [isEditing, setIsEditing] = useState(false);

  return (
    <div className="flex flex-1 flex-col w-full items-center gap-14 justify-center">
      {isEditing ? (
        <SpotEditAccount goBack={() => setIsEditing(false)} />
      ) : (
        <>
          <AccountInfo
            avatarUri="/images/avatars/spot.svg"
            triggerEdit={() => setIsEditing(true)}
          />
          <SpotPortfolioTable
            sendAction={() => navigate("/transfer?action=send")}
            receiveAction={() => navigate("/transfer?action=receive")}
          />
        </>
      )}
    </div>
  );
};
