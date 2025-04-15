import { type AppletMetadata, IconAddCross } from "@left-curve/applets-kit";
import { useStorage } from "@left-curve/store";
import { useApp } from "~/hooks/useApp";

import { applets } from "../../../applets";

import { Link } from "@tanstack/react-router";

export function AppletsSection() {
  const [favApplets] = useStorage<AppletMetadata[]>("fav_applets", {
    initialValue: applets.slice(0, 3),
  });

  const { setSearchBarVisibility } = useApp();

  return (
    <div className="grid grid-cols-[repeat(auto-fill,_minmax(64px,_1fr))] md:place-items-start gap-4 md:gap-8 w-full min-h-[40vh] md:min-h-fit">
      {favApplets.map((applet) => (
        <div key={applet.title} className="flex flex-col items-center gap-2">
          <Link
            to={applet.path}
            className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-red-bean-50 rounded-xl p-[10px]"
          >
            <img src={applet.img} alt={applet.title} className="w-full h-full" />
          </Link>
          <p className="text-sm font-bold">{applet.title}</p>
        </div>
      ))}

      <div className="flex flex-col items-center gap-2">
        <button
          type="button"
          onClick={() => setSearchBarVisibility(true)}
          className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow border-[1.43px] border-rice-100 text-rice-100 rounded-xl p-[10px] flex items-center justify-center cursor-pointer"
        >
          <IconAddCross />
        </button>
        <p className="min-h-6" />
      </div>
    </div>
  );
}
