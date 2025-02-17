import { IconChevronRight, twMerge, useClickAway } from "@left-curve/applets-kit";
import type React from "react";
import { useRef, useState } from "react";

import { Link } from "@tanstack/react-router";
import { motion } from "framer-motion";

interface Props {
  showAccountMenu: boolean;
  setShowAccountMenu: (v: boolean) => void;
}

export const AccountMenu: React.FC<Props> = ({ showAccountMenu, setShowAccountMenu }) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuAccountActiveLink, setMenuAccountActiveLink] = useState<"Assets" | "Earn" | "Pools">(
    "Assets",
  );

  useClickAway(menuRef, () => setShowAccountMenu(false));
  return (
    <div
      ref={menuRef}
      className={twMerge(
        "transition-all fixed top-0 flex h-[100vh] justify-end z-50 duration-300 delay-100 w-full md:max-w-[422px] bg-[linear-gradient(90deg,_rgba(0,_0,_0,_0)_3.2%,_rgba(46,_37,_33,_0.1)_19.64%,_rgba(255,_255,_255,_0.1)_93.91%)]",
        showAccountMenu ? "right-0" : "right-[-100vh]",
      )}
    >
      <div
        className="text-gray-900 h-full py-6 flex justify-end w-[64px] hover:cursor-pointer pr-1"
        onClick={() => setShowAccountMenu(false)}
      >
        <IconChevronRight />
      </div>
      <div className="pr-2 py-4 w-full">
        <div className="w-full bg-white-100 flex flex-col items-center h-full rounded-2xl border border-gray-100">
          <div className="p-4 w-full flex items-center flex-col gap-5">
            {/* card component */}
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
            {/*  buttons */}
            <div className="md:self-end flex gap-4 items-center justify-center w-full">
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
            {/* Links component */}
            <motion.ul className="flex gap-4 text-base relative border-b border-b-gray-100 w-full items-center">
              {Array.from(["Assets", "Earn", "Pools"]).map((e, i) => {
                const isActive = e === menuAccountActiveLink;
                return (
                  <motion.li
                    className="relative px-4 transition-all flex-1 flex items-center justify-center py-3"
                    key={`navLink-${e}`}
                    onClick={() => setMenuAccountActiveLink(e as any)}
                  >
                    <Link
                      className={twMerge(
                        "italic font-medium font-exposure transition-all",
                        isActive ? "text-red-bean-400" : "text-gray-300",
                      )}
                    >
                      {e}
                    </Link>
                    {isActive ? (
                      <motion.div
                        className="w-full h-[1px] bg-red-bean-400 absolute bottom-0 left-0"
                        layoutId="underline"
                      />
                    ) : null}
                  </motion.li>
                );
              })}
            </motion.ul>
          </div>

          {/* Menus component */}
          {menuAccountActiveLink === "Assets" ? (
            <div className="flex flex-col w-full overflow-y-auto scrollbar-none pb-4">
              {Array.from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]).map((e, i) => {
                return (
                  <div
                    key={`asset-${e}`}
                    className="flex items-center justify-between p-4 hover:bg-rice-50"
                  >
                    <div className="flex gap-2 items-center">
                      <img
                        src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                        alt=""
                        className="rounded-full h-8 w-8"
                      />
                      <div className="flex flex-col text-base">
                        <p className="text-gray-500">Ethereum</p>
                        <p>$124.05</p>
                      </div>
                    </div>
                    <div className="flex flex-col">
                      <p className="text-gray-500">2,09%</p>
                      <p>$1200.05</p>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
};
