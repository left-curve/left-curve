import { Button, Carousel, ResizerContainer, twMerge } from "@left-curve/applets-kit";
import { useAccount, useStorage } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import type React from "react";
import { type PropsWithChildren, useEffect } from "react";
import { m } from "~/paraglide/messages";

interface Props {
  isFirstVisit: boolean;
}

export const LoginWrapper: React.FC<PropsWithChildren<Props>> = ({
  children,
  isFirstVisit: firstVisit,
}) => {
  const { isConnected } = useAccount();
  const navigate = useNavigate();

  const [isFirstVisit, setIsFirstVisit] = useStorage<boolean>("firstVisit", {
    initialValue: firstVisit,
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
        <div className="bg-[url('./images/frame-rounded-mobile.svg')] min-h-[5rem] h-[5rem] w-full bg-no-repeat bg-[center_-1.5rem] xl:hidden" />
        <Carousel className="gap-2 sm:gap-4 xl:gap-6 w-full py-4 pb-6">
          <div className="flex flex-col items-center justify-between gap-4 md:gap-8 text-center px-4 xl:px-0 h-full md:pb-0 pb-[120px] md:pt-auto md:pb-auto">
            <img
              src="/images/characters/birdo.svg"
              alt="birdo"
              className="max-h-available object-contain"
              draggable={false}
            />
            <div className="flex flex-col items-center justify-center gap-1 min-h-fit max-w-full md:max-w-[25rem]">
              <h3 className="exposure-h3-italic">{m["signup.carousel.title"]({ step: 0 })}</h3>
              <p className="text-gray-500 text-xs sm:text-md">
                {m["signup.carousel.description"]({ step: 0 })}
              </p>
            </div>
          </div>
          <div className="flex flex-col items-center justify-between gap-4 md:gap-8 text-center px-4 xl:px-0 h-full md:pb-0 pb-[120px] md:pt-auto md:pb-auto">
            <img
              src="/images/characters/birdo.svg"
              alt="birdo"
              className="max-h-available object-contain"
              draggable={false}
            />
            <div className="flex flex-col items-center justify-center gap-1 min-h-fit max-w-full md:max-w-[25rem]">
              <h3 className="exposure-h3-italic">{m["signup.carousel.title"]({ step: 1 })}</h3>
              <p className="text-gray-500 text-xs sm:text-md">
                {m["signup.carousel.description"]({ step: 1 })}
              </p>
            </div>
          </div>
          <div className="flex flex-col items-center justify-between gap-4 md:gap-8 text-center px-4 xl:px-0 h-full md:pb-0 pb-[120px] md:pt-auto md:pb-auto">
            <img
              src="/images/characters/birdo.svg"
              alt="birdo"
              className="max-h-available object-contain"
              draggable={false}
            />
            <div className="flex flex-col items-center justify-center gap-1 min-h-fit max-w-full md:max-w-[25rem]">
              <h3 className="exposure-h3-italic">{m["signup.carousel.title"]({ step: 2 })}</h3>
              <p className="text-gray-500 text-xs sm:text-md">
                {m["signup.carousel.description"]({ step: 2 })}
              </p>
            </div>
          </div>
        </Carousel>
        <div className="h-[40px]" />
        <div className=" absolute w-full bottom-20 md:bottom-24 px-8 xl:hidden max-w-[25rem]">
          <Button variant="secondary" fullWidth onClick={() => setIsFirstVisit(false)}>
            Continue
          </Button>
        </div>
        <div className="bg-[url('./images/frame-rounded-mobile.svg')] min-h-[5rem] h-[5rem] w-full bg-no-repeat bg-[center_1rem] xl:hidden" />
      </div>
    </div>
  );
};
