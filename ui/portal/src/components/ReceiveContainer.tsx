import { QRCode, useAccountName } from "@dango/shared";
import { useAccount } from "@leftcurve/react";
import { truncateAddress } from "@leftcurve/utils";

export const ReceiveContainer: React.FC = () => {
  const { account } = useAccount();
  const [name] = useAccountName();

  const copyAction = () => {
    if (!account) return;
    navigator.clipboard.writeText(account.address);
  };

  return (
    <div className="dango-grid-5x5-M gap-4 flex flex-col items-center justify-center w-full text-typography-black-200">
      <div className="p-4 bg-surface-rose-200 rounded-full">
        <img src="/images/send-and-receive.webp" alt="transfer" className="w-[88px] h-[88px]" />
      </div>
      <div className="flex gap-8">
        <p className="uppercase font-extrabold text-2xl">{name}</p>
      </div>
      <div className="flex gap-4">
        <p className="font-black text-lg">{truncateAddress(account?.address!)}</p>
        <p className="cursor-pointer" onClick={copyAction}>
          Copy Address
        </p>
      </div>
      <QRCode data={account?.address!} />
    </div>
  );
};
