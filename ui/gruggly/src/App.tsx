import { IconClockBitcoin, IconFlame } from "@tabler/icons-react";

export const App: React.FC = () => {
  return (
    <>
      <div className="w-full">
        <div className="flex items-center justify-center flex-col">
          <div className="flex flex-col gap-[40px] py-11 items-center">
            <h1 className="text-2xl">Explore Dango Smart Contacts</h1>
            <div>Search Input</div>
          </div>
          <div className="py-6 grid grid-cols-2 gap-9 w-full border-t border-cw-grey-700">
            <h2 className="flex items-center gap-3 justify-start text-lg col-span-2">
              <IconClockBitcoin className="h-[25px]" />
              Latest smart contracts
            </h2>
            <div className="grid grid-cols-1 lg:grid-cols-2 col-span-2 gap-4" />
            <div className="grid grid-cols-1 gap-9 col-span-1">
              <h2 className="flex items-center gap-3 justify-start text-lg">
                <IconFlame className="h-[25px]" />
                Popular smart contracts
              </h2>
            </div>
          </div>
        </div>
      </div>
    </>
  );
};
