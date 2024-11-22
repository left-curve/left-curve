import { useParams } from "react-router-dom";
import { AccountRouter } from "~/components/AccountRouter";

const AccountView: React.FC = () => {
  const { index = "0" } = useParams<{ index: string }>();
  return (
    <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
      <div className="flex flex-1 flex-col items-center justify-center gap-4 w-full">
        <AccountRouter index={Number.parseInt(index)} />
      </div>
    </div>
  );
};

export default AccountView;
