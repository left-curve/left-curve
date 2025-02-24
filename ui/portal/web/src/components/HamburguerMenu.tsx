import {
  BellIcon,
  DangoDotsIcon,
  Hamburger,
  IconButton,
  IconGear,
  ProfileIcon,
  type VisibleRef,
  twMerge,
  useClickAway,
} from "@left-curve/applets-kit";
import { useNavigate } from "@tanstack/react-router";
import { forwardRef, useRef, useState } from "react";
import { useApp } from "~/hooks/useApp";

export const HamburgerMenu = forwardRef<VisibleRef>((_props, ref) => {
  const { setSidebarVisibility } = useApp();
  const [showOptions, setShowOptions] = useState(false);
  const navigate = useNavigate();
  const menuRef = useRef<HTMLDivElement>(null);

  useClickAway(menuRef, () => setShowOptions(false));

  return (
    <div
      ref={menuRef}
      className={twMerge("flex flex-col lg:hidden h-11 w-11 z-60 transition-all duration-300", {
        "bottom-[2.75rem]": window.matchMedia("(display-mode: standalone)").matches,
      })}
    >
      <IconButton
        variant="utility"
        size="lg"
        className={twMerge(
          "absolute",
          {
            "translate-y-[-15.25rem] shadow-btn-shadow-gradient": showOptions,
          },
          { "shadow-none": !showOptions },
        )}
        type="button"
        onClick={() => [setShowOptions(!showOptions), setSidebarVisibility(true)]}
      >
        <ProfileIcon className="h-6 w-6 " />
      </IconButton>

      <IconButton
        variant="utility"
        size="lg"
        className={twMerge(
          "absolute",
          {
            "translate-y-[-11.25rem] shadow-btn-shadow-gradient": showOptions,
          },
          { "shadow-none": !showOptions },
        )}
        type="button"
        onClick={() => [setShowOptions(!showOptions)]}
      >
        <BellIcon className="h-6 w-6 " />
      </IconButton>

      <IconButton
        variant="utility"
        size="lg"
        className={twMerge(
          "absolute",
          {
            "translate-y-[-7.5rem] shadow-btn-shadow-gradient": showOptions,
          },
          { "shadow-none": !showOptions },
        )}
        type="button"
        onClick={() => [setShowOptions(!showOptions), navigate({ to: "/settings" })]}
      >
        <IconGear className="h-6 w-6 " />
      </IconButton>

      <IconButton
        variant="utility"
        size="lg"
        className={twMerge(
          "absolute",
          {
            "translate-y-[-3.75rem] shadow-btn-shadow-gradient": showOptions,
          },
          { "shadow-none": !showOptions },
        )}
        type="button"
        onClick={() => [setShowOptions(!showOptions), navigate({ to: "/" })]}
      >
        <DangoDotsIcon className="h-6 w-6 " />
      </IconButton>

      <Hamburger isOpen={showOptions} onClick={() => setShowOptions(!showOptions)} />
    </div>
  );
});
