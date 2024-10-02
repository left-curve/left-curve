import { AccountInfo, SpotPortfolioTable } from "@leftcurve/ui";

export const ManageSpot: React.FC = () => {
  return (
    <>
      <AccountInfo avatarUri="/images/avatars/spot.png" />
      <SpotPortfolioTable />
    </>
  );
};
