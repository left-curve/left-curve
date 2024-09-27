import { AccountInfo, SpotPortfolioTable } from "@leftcurve/dango";

export const ManageSpot: React.FC = () => {
  return (
    <>
      <AccountInfo avatarUri="/images/avatars/spot.png" />
      <SpotPortfolioTable />
    </>
  );
};
