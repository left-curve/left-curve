import type { ComponentPropsWithoutRef } from "react";

export const Footer: React.FC<ComponentPropsWithoutRef<"footer">> = ({ className, ...props }) => {
  return (
    <footer className={"flex flex-col gap-10 items-center justify-center " + className} {...props}>
      <div className="flex gap-12 uppercase font-extrabold">
        <a href="https://x.com/leftCurveSoft" target="_blank" rel="noreferrer">
          X
        </a>
        <a href="/">DISCORD</a>
      </div>
      <div className="flex items-center justify-between md:justify-center text-xs font-light md:gap-12 px-4 w-full">
        <a href="/">TERMS OF USE</a>
        <a href="/">COOKIE POLICY</a>
        <a href="/">PRIVACY POLICY</a>
      </div>
    </footer>
  );
};
