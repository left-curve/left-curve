import type React from "react";

export const Kiosk: React.FC = () => {
  return (
    <div className="w-full h-full transition-all hover:scale-110">
      <img
        src="/images/kiosko.svg"
        alt="kisko"
        className="object-fit w-full h-full transition-all kiosko"
      />
    </div>
  );
};
