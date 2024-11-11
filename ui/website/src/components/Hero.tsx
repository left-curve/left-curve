import type React from "react";
import { Dancer } from "./Dancer";
import { Dog } from "./Dog";
import { Kiosk } from "./Kiosk";
import { Monkeys } from "./Monkeys";
import { Rabbits } from "./Rabbits";
import { Snake } from "./Snake";

interface Props {
  goSectionBelow: () => void;
}

export const Hero: React.FC<Props> = ({ goSectionBelow }) => {
  return (
    <div className="section justify-center items-center h-screen w-screen overflow-x-hidden">
      <div className="w-screen h-screen flex flex-col items-center justify-evenly px-2 z-10 flex-1 relative pt-24 md:pb-4">
        <div className="h-[60%] md:h-[80%] 3xl:h-[90%] w-full relative max-w-[1240px] 3xl:max-w-[1440px] z-20 mx-auto">
          <Rabbits />
          <Monkeys />
          <Dancer />
          <Kiosk />
          <Dog />
          <Snake />
        </div>
        <button
          type="button"
          onClick={goSectionBelow}
          className="text-typography-purple-300 hover:text-typography-purple-400 cursor-pointer"
        >
          <svg
            width="32"
            height="32"
            viewBox="0 0 32 32"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M2.85712 9.42859C5.80283 15.32 8.74856 18.5706 14.0619 21.5478C15.2649 22.2219 16.7351 22.2219 17.938 21.5478C23.2513 18.5706 26.1971 15.32 29.1428 9.42859"
              stroke="currentColor"
              strokeWidth="2.66667"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
      </div>
    </div>
  );
};
