import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

import { IconAddCross, twMerge } from "@left-curve/applets-kit";
import { motion } from "framer-motion";

export const Route = createFileRoute("/(app)/_app/")({
  component: OverviewComponent,
});

function OverviewComponent() {
  const [tableActive, setTableActive] = useState<"Assets" | "Earn" | "Pools">("Assets");
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
              className="flex-1 w-full [box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A] border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.12)_8%,_rgba(46,_37,_33,_0.24)_100%)] bg-red-bean-400 px-4 py-2 rounded-full font-exposure text-red-bean-50 italic font-medium"
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
      {/* fourth component */}
      <div className="bg-rice-25 shadow-card-shadow flex flex-col rounded-medium w-full p-4 gap-4">
        {/* button components */}
        <motion.ul className="flex text-base relative  items-center w-fit bg-green-bean-200 p-1 rounded-small">
          {Array.from(["Assets", "Earn", "Pools"]).map((e, i) => {
            const isActive = e === tableActive;
            return (
              <motion.li
                className="relative transition-all flex items-center justify-center py-2 px-4 cursor-pointer"
                key={`navLink-${e}`}
                onClick={() => setTableActive(e as any)}
              >
                <p
                  className={twMerge(
                    "italic font-medium font-exposure transition-all relative z-10",
                    isActive ? "text-black" : "text-gray-300",
                  )}
                >
                  {e}
                </p>
                {isActive ? (
                  <motion.div
                    className="w-full h-full rounded-[10px] bg-green-bean-50 absolute bottom-0 left-0 [box-shadow:0px_4px_6px_2px_#1919191F]"
                    layoutId="active"
                  />
                ) : null}
              </motion.li>
            );
          })}
        </motion.ul>
        {/*  table component */}
        <div className="overflow-y-auto scrollbar-none w-full">
          <table className="table-auto w-full">
            {/* Header */}
            <thead>
              <tr className=" text-[#717680] font-semibold text-xs">
                <th className="text-start rounded-l-xl p-4 bg-green-bean-100">Vault</th>
                <th className="text-end bg-green-bean-100 p-4">Type</th>
                <th className="text-end bg-green-bean-100 p-4">APR</th>
                <th className="text-end bg-green-bean-100 p-4">
                  <p className="min-w-fit">Liquidity Available</p>
                </th>
                <th className="text-end bg-green-bean-100 p-4">TVL</th>
                <th className="text-end rounded-r-xl bg-green-bean-100 p-4">Risk Level</th>
              </tr>
            </thead>
            {/* <div className="grid grid-cols-6 gap-4 p-4 rounded-small bg-green-bean-100 text-[#717680] font-semibold">
              <p className="text-xs text-gray-500">Vault</p>
              <p className="text-xs text-gray-500 flex items-center justify-end">Type</p>
              <p className="text-xs text-gray-500 flex items-center justify-end">APR</p>
              <p className="text-xs text-gray-500 flex items-center justify-end">
                Liquidity Available
              </p>
              <p className="text-xs text-gray-500 flex items-center justify-end">TVL</p>
              <p className="text-xs text-gray-500 text-end">Risk Level</p>
            </div> */}

            {/* rows */}
            <tbody>
              {Array.from([1, 2, 3, 4, 5]).map((e) => {
                return (
                  <tr className="p-4 border-b border-b-gray-100" key={`row-${e}`}>
                    <td className="p-4">
                      <div className="flex gap-2 text-lg">
                        <div className="flex">
                          <img
                            src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                            alt=""
                            className="h-6 min-w-6 rounded-full"
                          />
                          <img
                            src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                            alt=""
                            className="h-6 min-w-6 -ml-1 rounded-full"
                          />
                        </div>
                        <p className="min-w-fit">ETH-USD</p>
                      </div>
                    </td>
                    <td className="p-4">
                      <div className="flex items-center justify-end">
                        <div className="text-xs bg-green-bean-200 border border-green-bean-300 text-green-bean-700 py-1 px-2 rounded-[4px] h-fit w-fit">
                          Lending
                        </div>
                      </div>
                    </td>
                    <td className="p-4">
                      <div className="flex items-center justify-end">17.72%</div>
                    </td>
                    <td className="p-4">
                      <div className="flex items-center justify-end">15.63%</div>
                    </td>
                    <td className="p-4">
                      <div className="flex items-center justify-end">15.63%</div>
                    </td>
                    <td className="p-4">
                      <div className="flex items-center justify-end">Low</div>
                    </td>
                  </tr>
                  /*  <div
                    className="grid grid-cols-6 gap-4 p-4 border-b border-b-gray-100"
                    key={`row-${e}`}
                  >
                    <div className="flex">
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
                    </div>
                    <div className="flex items-center justify-end">
                      <div className="text-xs bg-green-bean-200 border border-green-bean-300 text-green-bean-700 py-1 px-2 rounded-[4px] h-fit w-fit">
                        Lending
                      </div>
                    </div>
                    <div className="flex items-center justify-end">17.72%</div>
                    <div className="flex items-center justify-end">15.63%</div>
                    <div className="flex items-center justify-end">15.63%</div>
                    <div className="flex items-center justify-end">Low</div>
                  </div> */
                );
              })}
            </tbody>
          </table>
        </div>
        {/* button view all */}
        <button
          type="button"
          className="self-center bg-blue-50 text-blue-500 w-fit italic py-2 px-4 font-exposure font-medium rounded-full [box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A] border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.12)_8%,_rgba(46,_37,_33,_0.24)_100%)]"
        >
          View All
        </button>
      </div>
    </div>
  );
}
