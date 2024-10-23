import { GradientContainer, QRCode } from "@dango/shared";
import { useAccount } from "@leftcurve/react";
import { truncateAddress } from "@leftcurve/utils";

export const ReceiveContainer: React.FC = () => {
  const { account } = useAccount();
  return (
    <GradientContainer className="gap-4 justify-center w-full min-h-[37.5rem] text-typography-black-200">
      <div className="p-4 bg-surface-rose-200 rounded-full">
        <img src="/images/send-and-receive.webp" alt="transfer" className="w-[88px] h-[88px]" />
      </div>
      <div className="flex gap-8">
        <p className="uppercase font-extrabold text-2xl">{account?.username}</p>
        <p className="uppercase font-extrabold text-2xl">
          {account?.type} #{account?.index}
        </p>
      </div>
      <div className="flex gap-4">
        <p className="font-black text-lg">{truncateAddress(account?.address!)}</p>
        <p
          className="cursor-pointer"
          onClick={() => navigator.clipboard.writeText(account?.address!)}
        >
          Copy Address
        </p>
      </div>
      <QRCode data={account?.address!} />
    </GradientContainer>
  );
};
