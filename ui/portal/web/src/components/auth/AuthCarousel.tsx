import { twMerge, useTheme } from "@left-curve/applets-kit";

import { Button, Carousel } from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";

import type React from "react";
import { useApp } from "~/hooks/useApp";

export const AuthCarousel: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { isFirstVisit } = settings;
  const { theme } = useTheme();

  const isDarkTheme = theme === "dark";

  return (
    <div
      className={twMerge(
        "custom-width h-svh xl:min-w-[720px] xl:w-[720px] bg-surface-primary-rice overflow-hidden",
        "items-start xl:pt-0 xl:items-center justify-center",
        " bg-no-repeat bg-cover bg-center",
        isDarkTheme
          ? "xl:bg-[url('./images/dark-frame-rounded.svg')]"
          : "xl:bg-[url('./images/frame-rounded.svg')]",
        isFirstVisit
          ? "fixed z-30 top-0 left-0 flex xl:relative w-screen gap-4 justify-between items-center flex-col xl:flex-row"
          : "hidden xl:flex",
      )}
    >
      <div
        className={twMerge(
          "min-h-[5rem] h-[5rem] w-full bg-no-repeat bg-[center_-1.5rem] xl:hidden",
          isDarkTheme
            ? "bg-[url('./images/dark-frame-rounded-mobile.svg')]"
            : "bg-[url('./images/frame-rounded-mobile.svg')]",
        )}
      />
      <Carousel className="gap-2 sm:gap-4 xl:gap-6 w-full h-full md:max-h-[60%]">
        {["stonk", "leverage", "smaug"].map((img, index) => {
          const title = m["signup.carousel.title"]({ step: index });
          return (
            <div
              key={title}
              className="flex flex-col items-center justify-between gap-4 md:gap-8 text-center px-4 xl:px-0 h-full max-h-full"
            >
              <div className="flex flex-1 w-full overflow-hidden">
                <img
                  src={`/images/carousel/${img}.svg`}
                  alt={title}
                  className="object-contain w-full h-full"
                  draggable={false}
                />
              </div>
              <div className="flex flex-col flex-1 items-center justify-center gap-1 max-w-full md:max-w-[25rem]">
                <h3 className="exposure-h3-italic">{title}</h3>
                <p className="text-tertiary-500 text-sm sm:text-md">
                  {m["signup.carousel.description"]({ step: index })}
                </p>
              </div>
            </div>
          );
        })}
      </Carousel>
      <div className="w-full block md:absolute bottom-[4.5rem] md:bottom-24 px-8 xl:hidden max-w-[25rem]">
        <Button
          variant="secondary"
          fullWidth
          onClick={() => changeSettings({ isFirstVisit: false })}
        >
          Continue
        </Button>
      </div>
      <div
        className={twMerge(
          "min-h-[5rem] h-[5rem] w-full bg-no-repeat bg-[center_1rem] xl:hidden",
          isDarkTheme
            ? "bg-[url('./images/dark-frame-rounded-mobile.svg')]"
            : "bg-[url('./images/frame-rounded-mobile.svg')]",
        )}
      />
    </div>
  );
};
