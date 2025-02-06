import { createRoute } from "@tanstack/react-router";
import { AppRoute } from "~/AppRouter";
import { IconAddCross } from "../../../../applets/kit/src/components/icons/IconAddCross";

export const OverviewRoute = createRoute({
  getParentRoute: () => AppRoute,
  path: "/",
  component: () => {
    return (
      <div className="w-full  md:max-w-[76rem] mx-auto flex flex-col gap-8 p-4">
        {/* first component */}
        <div className="rounded-medium bg-rice-50 shadow-card-shadow flex flex-col md:flex-row gap-4 w-full p-4 items-center md:items-start">
          {/* account card component */}
          <div className="shadow-account-card w-full max-w-[20.5rem] h-[9.75rem] bg-account-card-red relative overflow-hidden rounded-small flex flex-col justify-between p-4">
            <img
              src="/images/account-card/dog.svg"
              alt="account-card-dog"
              className="absolute right-0 bottom-0"
            />
            <div className="flex gap-1">
              <div className="flex flex-col">
                <p className="font-exposure text-base italic font-medium">Spot #123,456</p>
                <p className="text-xs text-neutral-500">0x6caf...FE09</p>
              </div>
              {/* badge component */}
              <div className="text-xs bg-blue-100 text-blue-800 py-1 px-2 rounded-full h-fit w-fit">
                Spot
              </div>
            </div>
            <div className="flex gap-2 items-center">
              <p className="text-xl ">125.04M</p>
              <p className="text-sm text-[#25B12A]">0.05%</p>
            </div>
          </div>
          <div className="w-full flex flex-col gap-4 items-center">
            {/*  assets component */}
            <div className="hidden md:flex flex-col bg-rice-25 [box-shadow:0px_-1px_2px_0px_#F1DBBA80,_0px_2px_4px_0px_#AB9E8A66] rounded-small p-4 gap-4 w-full">
              <div className="flex items-center justify-between w-full">
                <p className="text-base font-bold">Assets</p>
                <a href="/" className="font-exposure italic text-blue-500 font-medium">
                  View all
                </a>
              </div>
              <div className="flex flex-wrap gap-4 items-center justify-between">
                {/* Assets item component */}
                {Array.from([1, 2, 3, 4, 5]).map((e, i) => {
                  return (
                    <div className="flex gap-2 items-center" key={`asset-${e}`}>
                      <img
                        src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                        alt=""
                        className="rounded-full h-7 w-7"
                      />
                      <div className="flex flex-col text-xs">
                        <p>Ethereum</p>
                        <p className="text-gray-500">$124.05</p>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
            {/*  buttons */}
            <div className="md:self-end flex gap-4 items-center justify-center w-full md:max-w-[256px]">
              <button
                type="button"
                className="flex-1 w-full [box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A] border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.12)_8%,_rgba(46,_37,_33,_0.24)_100%)] bg-red-bean-400 px-4 py-2 rounded-full font-exposure text-white italic font-medium"
              >
                Fund
              </button>
              <button
                type="button"
                className="flex-1 w-full [box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A] border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(0,_0,_0,_0.04)_8%,_rgba(0,_0,_0,_0.07)_100%)] bg-blue-50 px-4 py-2 rounded-full font-exposure
            italic font-medium text-blue-500"
              >
                Send
              </button>
            </div>
          </div>
        </div>
        {/* second component */}
        <div className="flex gap-4 md:gap-8 items-start flex-wrap md:justify-start w-full">
          {/* applets items */}
          <div className="flex flex-col items-center gap-2">
            <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-green-bean-50 rounded-small p-[10px]">
              <img src="/images/applets/swap.svg" alt="" className="w-full h-full" />
            </div>
            <p className="text-sm font-bold">Swap</p>
          </div>
          <div className="flex flex-col items-center gap-2">
            <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-red-bean-50 rounded-small p-[10px]">
              <img src="/images/applets/earn.svg" alt="" className="w-full h-full" />
            </div>
            <p className="text-sm font-bold">Earn</p>
          </div>
          <div className="flex flex-col items-center gap-2">
            <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-rice-50 rounded-small p-[10px]">
              <img src="/images/applets/multisig.svg" alt="" className="w-full h-full" />
            </div>
            <p className="text-sm font-bold">Multisig</p>
          </div>
          {/* add applets item */}
          <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow border-[1.43px] border-rice-100 text-rice-100 rounded-small p-[10px] flex items-center justify-center">
            <IconAddCross />
          </div>
        </div>
        {/* third component */}
        <div className="bg-rice-25 shadow-card-shadow flex flex-col rounded-medium w-full">
          <p className="text-2xl font-extrabold px-4 py-3">Top Yields</p>

          <div className="flex gap-6 w-full overflow-y-scroll p-4 scrollbar-none">
            {/*  strategy cards */}
            {Array.from([1, 2, 3]).map((e) => {
              return (
                <div
                  className="relative p-4  min-h-[8.5rem] min-w-[17.375rem] bg-rice-50 shadow-card-shadow rounded-2xl overflow-hidden"
                  key={`item-${e}`}
                >
                  <img
                    src="/images/strategy-card/cocodrile.svg"
                    alt=""
                    className="absolute z-0 bottom-0 right-0 "
                  />
                  <div className="flex flex-col gap-2 justify-between z-10 w-full h-full relative">
                    <div className="flex flex-col gap-2">
                      <div className="flex gap-2 text-lg">
                        <div className="flex">
                          <img
                            src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                            alt=""
                            className="h-6 w-6 rounded-full"
                          />
                          <img
                            src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                            alt=""
                            className="h-6 w-6 -ml-1 rounded-full"
                          />
                        </div>
                        <p>ETH-USD</p>
                      </div>
                      <div className="text-xs bg-green-bean-200 text-gray-500 py-1 px-2 rounded-[4px] h-fit w-fit">
                        Stable Strategy
                      </div>
                    </div>
                    <div className="p-2 rounded-xl bg-rice-100 flex items-center justify-between text-xs">
                      <div className="flex gap-2">
                        <span className="text-gray-500">APY</span>
                        <span>17.72%</span>
                      </div>
                      <div className="flex gap-2">
                        <span className="text-gray-500">TVL</span>
                        <span>15.63%</span>
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    );
  },
});
