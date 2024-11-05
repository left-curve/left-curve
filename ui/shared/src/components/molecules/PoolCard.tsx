interface Props {
  onClickPool: (id: string) => void;
}

export const PoolCard: React.FC<Props> = ({ onClickPool }) => {
  return (
    <div
      className="py-4 px-6 items-center gap-1 grid grid-cols-[1fr_80px_80px_80px] text-end
            bg-surface-rose-100 hover:bg-surface-off-white-200 border-2 border-surface-off-white-500
          text-typography-black-100 hover:text-typography-black-300 rounded-2xl transition-all cursor-pointer font-normal leading-5"
      onClick={() => onClickPool("1")}
    >
      <div className="flex gap-3 items-center">
        <div className="flex">
          <img
            src="https://raw.githubusercontent.com/cosmos/chain-registry/master/_non-cosmos/ethereum/images/usdc.svg"
            alt="usdc"
            className="w-6 h-6 z-10"
          />
          <img
            src="https://raw.githubusercontent.com/cosmos/chain-registry/master/_non-cosmos/ethereum/images/wsteth.svg"
            alt="wseth"
            className="w-6 h-6 ml-[-0.5rem]"
          />
        </div>
        <p>USDC - stETH</p>
      </div>
      <p>$192.08k</p>
      <p>$192.08k</p>
      <p>1.20%</p>
    </div>
  );
};
