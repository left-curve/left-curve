import { IconAddCross } from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

import { useFavApplets } from "~/hooks/useFavApplets";

import { Link } from "@tanstack/react-router";

export function AppletsSection() {
  const { favApplets } = useFavApplets();
  const { setSearchBarVisibility } = useApp();

  return (
    <div className="grid grid-cols-[repeat(auto-fill,_minmax(80px,_1fr))] md:flex md:flex-wrap gap-4 md:gap-4 w-full md:min-h-fit md:justify-start">
      {Object.values(favApplets).map((applet) => (
        <div
          className="md:w-[5.5rem] h-auto flex items-start justify-center [&:nth-child(6n)]:md:justify-end [&:nth-child(6n+1)]:md:justify-start"
          key={`applets.section.${applet.title}}`}
        >
          <div className="flex flex-col items-center gap-2">
            <Link
              to={applet.path}
              className="h-16 w-16 md:h-20 md:w-20 shadow-account-card bg-primary-red hover:bg-surface-secondary-red transition-all rounded-xl p-[10px]"
            >
              <img src={applet.img} alt={applet.title} className="w-full h-full" />
            </Link>
            <p className="text-sm font-bold text-center">{applet.title}</p>
          </div>
        </div>
      ))}

      <div className="flex flex-col items-center md:items-start gap-2 md:w-[5.5rem]">
        <button
          type="button"
          onClick={() => setSearchBarVisibility(true)}
          className="h-16 w-16 md:h-20 md:w-20 shadow-account-card border-[1.43px] border-tertiary-rice text-tertiary-rice bg-foreground-primary-rice hover:bg-foreground-secondary-rice transition-all rounded-xl p-[10px] flex items-center justify-center cursor-pointer"
        >
          <IconAddCross />
        </button>
        <p className="min-h-6" />
      </div>
    </div>
  );
}
