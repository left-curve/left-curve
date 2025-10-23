import type React from "react";
import type { PropsWithChildren } from "react";

export const HeaderExplorer: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="w-full flex flex-col bg-surface-secondary-rice shadow-account-card overflow-hidden rounded-xl p-4 pt-6 mb-16 min-h-[33.75rem] items-center justify-center gap-6">
      <img
        src="/images/emojis/simple/map.svg"
        alt="map-emoji"
        className="max-w-full h-[154px] object-contain w-auto"
      />
      {children}
    </div>
  );
};
