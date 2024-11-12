import { DiscordIcon } from "./icons/Discord";
import { XBrandIcon } from "./icons/XBrand";

export const Footer: React.FC = () => {
  return (
    <footer className="flex flex-col gap-6 md:gap-10 items-center justify-center py-2 pb-12 md:pb-8">
      <div className="flex gap-12 uppercase font-extrabold">
        <a
          href="https://x.com/leftCurveSoft"
          target="_blank"
          rel="noreferrer"
          className="text-typography-black-300 hover:bg-typography-purple-300/40 rounded-full p-2 transition-all"
        >
          <XBrandIcon />
        </a>
        <a
          href="https://discord.gg/4uB9UDzYhz"
          target="_blank"
          rel="noreferrer"
          className="text-typography-black-300 hover:bg-typography-purple-300/40 rounded-full p-2 transition-all"
        >
          <DiscordIcon />
        </a>
      </div>
      {/*  <div
    className="flex items-center justify-between md:justify-center text-xs font-light md:gap-12 px-4 w-full"
  >
    <a href="/" className="uppercase">terms of use</a>
    <a href="/" className="uppercase">cookie policy</a>
    <a href="/" className="uppercase">privacy policy</a>
  </div> */}
    </footer>
  );
};
