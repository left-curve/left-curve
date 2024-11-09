import type React from "react";

export const Snake: React.FC = () => {
  return (
    <div className="w-full h-full transition-all hover:scale-110">
      <img
        src="/images/snake.svg"
        alt="snake"
        className="object-fit w-full h-full transition-all snake"
      />
    </div>
  );
};
