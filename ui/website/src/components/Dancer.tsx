import type React from "react";

export const Dancer: React.FC = () => {
  return (
    <div className="scale-x-[-1] md:scale-x-100 w-full h-full hover:scale-x-[-1.1] hover:scale-y-110 transition-all md:hover:scale-110">
      <img
        src="/images/dancer.svg"
        alt="dancer"
        className="object-fit w-full h-full transition-all dancer"
      />
    </div>
  );
};
