import { AccountInfo, SpotPortfolioTable } from "@dango/shared";

export const ManageSpot: React.FC = () => {
  return (
    <>
      <AccountInfo avatarUri="/images/avatars/spot.png" />
      <SpotPortfolioTable />
    </>
  );
};
