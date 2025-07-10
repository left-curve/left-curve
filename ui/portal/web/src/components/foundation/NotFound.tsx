import { useNavigate } from "@tanstack/react-router";

import { Button } from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";

export const NotFound: React.FC = () => {
  const navigate = useNavigate();
  return (
    <div className="w-full flex flex-1 justify-center items-center p-4 flex-col gap-6 text-center pb-[76px]">
      <img
        src="/images/characters/emptybox1.svg"
        alt="404 Not Found"
        className="w-full max-w-[14.75rem] md:max-w-[22.5rem] opacity-60"
      />
      <div className="flex flex-col gap-2">
        <h1 className="text-center font-exposure text-[30px] md:text-[60px] font-extrabold text-gray-700 italic">
          {m["notFound.title"]()}
        </h1>
        <p className="text-tertiary-500 diatype-m-regular">{m["notFound.description"]()}</p>
      </div>
      <Button onClick={() => navigate({ to: "/" })}>{m["notFound.goToHome"]()}</Button>
    </div>
  );
};
