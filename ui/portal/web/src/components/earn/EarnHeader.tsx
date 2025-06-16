import type React from "react";

import { m } from "~/paraglide/messages";

export const EarnHeader: React.FC = () => {
  return (
    <div className="flex flex-col items-center justify-center pb-6 text-center">
      <img
        src="/images/emojis/detailed/pig.svg"
        alt="pig-detailed"
        className="w-[148px] h-[148px]"
      />
      <h1 className="exposure-h1-italic text-gray-900">{m["earn.title"]()}</h1>
      <p className="text-gray-500 diatype-lg-medium">{m["earn.description"]()}</p>
    </div>
  );
};
