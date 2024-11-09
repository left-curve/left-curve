import type React from "react";

export const Dog: React.FC = () => {
  return (
    <div className="w-full h-full transition-all hover:scale-110">
      <img
        src="/images/dog.svg"
        alt="dog"
        className="object-fit w-full h-full transition-all dog"
      />
    </div>
  );
};
