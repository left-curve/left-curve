import { AccountInfo, SpotPortfolioTable } from "../../../packages/ui/build/index.mjs";

export const ManageSpot: React.FC = () => {
  return (
    <>
      <AccountInfo avatarUri="/images/avatars/spot.png" />
      <SpotPortfolioTable />
    </>
  );
};
