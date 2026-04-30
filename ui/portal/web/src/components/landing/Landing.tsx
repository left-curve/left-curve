import { useMediaQuery, useTheme } from "@left-curve/applets-kit";
import { SearchMenu } from "../foundation/SearchMenu";
import { AppletsSection } from "./AppletsSection";

export function Landing() {
  const { isLg } = useMediaQuery();
  const { theme } = useTheme();

  return (
    <div className="min-h-svh flex items-center justify-center relative w-full">
      <div className="min-h-[calc(100svh-5svh)] mx-auto pb-[20svh] p-4 lg:p-0 w-full flex flex-col gap-6 relative flex-1 items-center justify-between lg:items-center lg:justify-center lg:gap-16 lg:pb-60">
        <img
          src={`/images/dango${theme === "dark" ? "-dark" : ""}.svg`}
          alt="Dango"
          className="max-w-[10rem] lg:max-w-[13rem] select-none"
        />
        {isLg && (
          <div className="relative w-full h-11 z-40 max-w-[38rem]">
            <SearchMenu />
          </div>
        )}
        <div className="flex w-full max-w-[38rem]">
          <AppletsSection />
        </div>
      </div>
    </div>
  );
}
