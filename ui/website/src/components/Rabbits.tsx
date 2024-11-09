import type React from "react";

export const Rabbits: React.FC = () => {
  return (
    <div className="w-full h-full transition-all hover:scale-110">
      <img
        src="/images/rabbits.svg"
        alt="rabbits"
        className="object-fit w-full h-full transition-all rabbit"
      />
    </div>
  );
};
