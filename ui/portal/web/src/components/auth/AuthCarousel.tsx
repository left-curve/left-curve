import { Carousel } from "@left-curve/applets-kit";

import type React from "react";

export const AuthCarousel: React.FC = () => {
  return (
    <div className="custom-width h-full min-w-[720px] w-[720px] hidden xl:flex bg-[url('./images/frame-rounded.svg')] bg-no-repeat bg-cover bg-center items-center justify-center">
      <Carousel className="gap-2 sm:gap-4 xl:gap-6 w-full flex-1 py-4">
        <div className="flex flex-col items-center justify-center gap-8 text-center px-4 xl:px-0  flex-1">
          <img
            src="/images/characters/birdo.svg"
            alt="birdo"
            className="w-full max-w-[14rem] sm:max-w-[22rem] h-auto object-contain"
            draggable={false}
          />
          <div className="flex flex-col items-center justify-center gap-1 max-w-full lg:max-w-[25rem]">
            <h3 className="exposure-h3-italic">Welcome home</h3>
            <p className="text-gray-500 text-md">The good old days are here to stay.</p>
          </div>
        </div>
        <div className="flex flex-col items-center justify-center gap-8 text-center px-4 xl:px-0  flex-1">
          <img
            src="/images/characters/birdo.svg"
            alt="birdo"
            className="w-full max-w-[14rem] sm:max-w-[22rem] h-auto object-contain"
            draggable={false}
          />
          <div className="flex flex-col items-center justify-center gap-1 max-w-full lg:max-w-[25rem]">
            <h3 className="exposure-h3-italic">Use Dango</h3>
            <p className="text-gray-500 text-md">
              Lorem ipsum dolor sit amet, consectetur adipiscing elit.
            </p>
          </div>
        </div>
        <div className="flex flex-col items-center justify-center gap-8 text-center px-4 xl:px-0  flex-1">
          <img
            src="/images/characters/birdo.svg"
            alt="birdo"
            className="w-full max-w-[14rem] sm:max-w-[22rem] h-auto object-contain"
            draggable={false}
          />
          <div className="flex flex-col items-center justify-center gap-1 max-w-full lg:max-w-[25rem]">
            <h3 className="exposure-h3-italic">How to use it</h3>
            <p className="text-gray-500 text-md">Fusce purus justo, lobortis aliquet orci.</p>
          </div>
        </div>
      </Carousel>
    </div>
  );
};
