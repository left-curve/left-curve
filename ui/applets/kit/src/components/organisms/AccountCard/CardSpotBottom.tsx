interface Props {
  isLoading: boolean;
  totalBalance: string;
}

export const CardSpotBottom: React.FC<Props> = ({ isLoading, totalBalance }) => {
  return (
    <div className="flex items-center justify-between">
      <p className="uppercase text-sm text-typography-rose-500 font-semibold">Balance:</p>
      <p className="text-2xl font-extrabold text-typography-rose-600">
        {isLoading ? "0" : totalBalance}
      </p>
    </div>
  );
};
