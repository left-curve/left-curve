import { IconDoubleChevronRight, twMerge, useClickAway } from "@left-curve/applets-kit";
import { useRef } from "react";
import { useApp } from "~/hooks/useApp";

import type React from "react";
import { AccountMenuBody } from "./AccountBody";

export const AccountDesktopMenu: React.FC = () => {
  const menuRef = useRef<HTMLDivElement>(null);
  const { setSidebarVisibility, isSidebarVisible } = useApp();

  useClickAway(menuRef, () => setSidebarVisibility(false));

  return (
    <div
      ref={menuRef}
      className={twMerge(
        "transition-all lg:absolute fixed top-0 flex h-[100vh] justify-end z-50 duration-300 delay-100 w-full lg:max-w-[422px] bg-[linear-gradient(90deg,_rgba(0,_0,_0,_0)_3.2%,_rgba(46,_37,_33,_0.1)_19.64%,_rgba(255,_255,_255,_0.1)_93.91%)]",
        isSidebarVisible ? "right-0" : "right-[-100vw]",
      )}
    >
      <div
        className="hidden group h-full py-4 lg:flex justify-end w-[84px] mr-[-20px]"
        onClick={() => setSidebarVisibility(false)}
      >
        <div className="h-full py-2 pr-8 group-hover:translate-x-2 pl-2 text-gray-500 cursor-pointer group-hover:bg-gray-300/20 rounded-tl-lg rounded-bl-lg transition-all">
          <IconDoubleChevronRight className="transition-all group-hover:scale-90" />
        </div>
      </div>
      <div className="lg:pr-2 lg:py-4 w-full relative z-10">
        <div className="w-full bg-white-100 flex flex-col items-center h-full rounded-t-2xl lg:rounded-2xl border border-gray-100">
          <AccountMenuBody />
        </div>
      </div>
    </div>
  );
};
