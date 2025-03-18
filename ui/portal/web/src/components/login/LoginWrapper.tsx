import { Birdo, Button, Carousel, ResizerContainer, twMerge } from "@left-curve/applets-kit";
import { useAccount, useStorage } from "@left-curve/store-react";
import { useNavigate } from "@tanstack/react-router";
import type React from "react";
import { useEffect } from "react";

export const LoginWrapper: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { isConnected } = useAccount();
  const navigate = useNavigate();

  const [isFirstVisit, setIsFirstVisit] = useStorage<boolean>("firstVisit", {
    initialValue: true,
  });

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <div className="h-svh xl:h-screen w-screen flex items-center justify-center">
      <div className="flex items-center justify-center flex-1">
        <ResizerContainer layoutId="login" className="w-full max-w-[22.5rem]">
          {children}
        </ResizerContainer>
      </div>
      <div
        className={twMerge(
          "custom-width xl:h-full xl:min-w-[720px] xl:w-[720px] bg-white-100 overflow-hidden",
          "items-start xl:pt-0 xl:items-center justify-center",
          "xl:bg-[url('./images/frame-rounded.svg')] bg-no-repeat bg-cover bg-center",
          isFirstVisit
            ? "fixed z-30 top-0 left-0 flex xl:relative w-screen h-svh gap-2 justify-between items-center flex-col xl:flex-row"
            : "hidden xl:flex",
        )}
      >
        <span className="bg-[url('./images/frame-rounded-mobile.svg')] h-[5rem] w-full  bg-no-repeat bg-[center_-1.5rem] xl:hidden" />
        <Carousel className="gap-2 sm:gap-4 xl:gap-6 w-full flex-1 py-4">
          <div className="flex flex-col items-center justify-center gap-8 text-center px-4 xl:px-0  flex-1">
            <img
              src="/images/characters/birdo.svg"
              alt="birdo"
              className="w-full max-w-[14rem] sm:max-w-[22rem] h-auto object-contain"
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
            />
            <div className="flex flex-col items-center justify-center gap-1 max-w-full lg:max-w-[25rem]">
              <h3 className="exposure-h3-italic">Use Dango</h3>
              <p className="text-gray-500 text-md">
                Lorem ipsum dolor sit amet, consectetur adipiscing elit. Lorem ipsum dolor sit amet,
                consectetur adipiscing elit.
              </p>
            </div>
          </div>
          <div className="flex flex-col items-center justify-center gap-8 text-center px-4 xl:px-0  flex-1">
            <img
              src="/images/characters/birdo.svg"
              alt="birdo"
              className="w-full max-w-[14rem] sm:max-w-[22rem] h-auto object-contain"
            />
            <div className="flex flex-col items-center justify-center gap-1 max-w-full lg:max-w-[25rem]">
              <h3 className="exposure-h3-italic">How to use it</h3>
              <p className="text-gray-500 text-md">Fusce purus justo, lobortis aliquet orci.</p>
            </div>
          </div>
        </Carousel>
        <div className="h-[40px]" />
        <div className=" absolute w-full bottom-24 px-8 xl:hidden max-w-[25rem]">
          <Button variant="secondary" fullWidth onClick={() => setIsFirstVisit(false)}>
            Continue
          </Button>
        </div>
        <span className="bg-[url('./images/frame-rounded-mobile.svg')] h-[5rem] w-full bg-no-repeat bg-[center_1rem] xl:hidden" />
      </div>
    </div>
  );
};
