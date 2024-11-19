import { AccountInfo, SpotEditAccount, SpotPortfolioTable } from "@dango/shared";
import { useState } from "react";
import { useNavigate } from "react-router-dom";

export const ManageSpot: React.FC = () => {
  const navigate = useNavigate();
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
