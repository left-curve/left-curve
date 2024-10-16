import { AccountInfo, SpotPortfolioTable } from "@dango/shared";
import { useNavigate } from "react-router-dom";

export const ManageSpot: React.FC = () => {
  const navigate = useNavigate();
  return (
    <>
      <AccountInfo avatarUri="/images/avatars/spot.png" />
      <SpotPortfolioTable
        navigate={navigate}
        sendUrl="/transfer?action=send"
        receiveUrl="/transfer?action=receive"
      />
    </>
  );
};
