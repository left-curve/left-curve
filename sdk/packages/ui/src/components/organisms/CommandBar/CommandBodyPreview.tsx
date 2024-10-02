"use client";

import { useStorage } from "@leftcurve/react";
import { AppletCard } from "~/components/molecules/AppletCard";

import type { AppletMetadata } from "~/types";

interface Props {
  popularApplets: AppletMetadata[];
  action: (applet: AppletMetadata) => void;
}

export const CommandBodyPreview: React.FC<Props> = ({ popularApplets, action }) => {
  const [recentApplets] = useStorage<AppletMetadata[]>("applets", {
    initialValue: [],
  });

  return (
    <>
      {recentApplets.length ? (
        <div className="py-2 flex flex-col gap-3 w-full">
          <h3 className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
            RECENT APPS
          </h3>

          <div className="flex w-full flex-col gap-1">
            {recentApplets.map((applet) => (
              <AppletCard key={crypto.randomUUID()} metadata={applet} onClick={action} />
            ))}
          </div>
        </div>
      ) : null}
      <div className="py-2 flex flex-col gap-3 w-full">
        <h3 className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
          POPULAR APPS
        </h3>
        <div className="flex w-full flex-col gap-1">
          {popularApplets.map((applet) => (
            <AppletCard key={crypto.randomUUID()} metadata={applet} onClick={action} />
          ))}
        </div>
      </div>
    </>
  );
};
