import type React from "react";

export const Monkeys: React.FC = () => {
  return (
    <div className="w-full h-full transition-all hover:scale-110">
      <img
        src="/images/monkeys.svg"
        alt="monkeys"
        className="object-fit w-full h-full transition-all monkey"
      />
    </div>
  );
};
