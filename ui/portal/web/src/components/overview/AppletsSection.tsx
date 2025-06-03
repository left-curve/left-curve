import { IconAddCross } from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

import { useFavApplets } from "~/hooks/useFavApplets";

import { Link } from "@tanstack/react-router";

export function AppletsSection() {
  const { favApplets } = useFavApplets();
  const { setSearchBarVisibility } = useApp();

  return (
    <div className="grid grid-cols-[repeat(auto-fill,_minmax(64px,_1fr))] md:place-items-start gap-4 md:gap-8 w-full min-h-[40vh] md:min-h-fit">
      {Object.values(favApplets).map((applet) => (
        <div key={applet.title} className="flex flex-col items-center gap-2">
          <Link
            to={applet.path}
            className="h-16 w-16 md:h-20 md:w-20 shadow-account-card bg-red-bean-50 hover:bg-red-bean-100 transition-all rounded-xl p-[10px]"
          >
            <img src={applet.img} alt={applet.title} className="w-full h-full" />
          </Link>
          <p className="text-sm font-bold text-center">{applet.title}</p>
        </div>
      ))}

      <div className="flex flex-col items-center gap-2">
        <button
          type="button"
          onClick={() => setSearchBarVisibility(true)}
          className="h-16 w-16 md:h-20 md:w-20 shadow-account-card border-[1.43px] border-rice-100 text-rice-100 hover:bg-rice-25 transition-all rounded-xl p-[10px] flex items-center justify-center cursor-pointer"
        >
          <IconAddCross />
        </button>
        <p className="min-h-6" />
      </div>
    </div>
  );
}
