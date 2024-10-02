interface Props {
  isLoading: boolean;
  totalBalance: string;
}

export const CardMarginBottom: React.FC<Props> = ({ isLoading, totalBalance }) => {
  return (
    <div className="flex flex-col flex-start">
      <p className="uppercase font-semibold text-typography-purple-400 text-[10px]">utilization:</p>
      <div className="flex items-center justify-between">
        <p className="text-typography-purple-400 text-sm font-extrabold">0%</p>
      </div>
      <div className="flex items-center justify-between">
        <p className="uppercase text-[10px] text-typography-purple-300 font-semibold">balance:</p>
        <p className="text-sm font-extrabold text-typography-purple-400">
          {isLoading ? "0" : totalBalance}
        </p>
      </div>
    </div>
  );
};
