import { twMerge, useTheme } from "@left-curve/applets-kit";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/maintenance")({
  component: MaintinanceApplet,
});

import { m } from "~/paraglide/messages";

function MaintinanceApplet() {
  const { theme } = useTheme();
  return (
    <div className="w-full flex flex-1 justify-center items-center p-4 flex-col gap-6 text-center pb-[76px] min-h-svh text-primary-900">
      <img
        src={`/images/union${theme === "dark" ? "-dark" : ""}.png`}
        alt="bg-image"
        className={twMerge(
          "drag-none select-none h-[15vh] lg:h-[20vh] w-full fixed lg:absolute bottom-0 lg:top-0 left-0 z-40 lg:z-0 rotate-180 lg:rotate-0",
        )}
      />
      <div className="fixed p-4 top-0 left-0 w-full">
        <div className="max-w-[76rem] mx-auto flex items-center justify-center w-full">
          <img
            src={`/images/dango${theme === "dark" ? "-dark" : ""}.svg`}
            alt="Dango"
            className="max-w-[10rem] lg:max-w-max"
          />
        </div>
      </div>
      <img
        src="/images/characters/grugo.svg"
        alt="Maintenance"
        className="w-full max-w-[14.75rem] md:max-w-[22.5rem] opacity-80"
      />
      <div className="flex flex-col gap-2 items-center justify-center">
        <h1 className="text-center font-exposure text-[30px] md:text-[60px] font-extrabold text-secondary-700 italic">
          {m["maintenance.title"]()}
        </h1>
        <p className="text-tertiary-500 diatype-m-regular max-w-xl">
          {m["maintenance.description"]()}
        </p>
      </div>
    </div>
  );
}
