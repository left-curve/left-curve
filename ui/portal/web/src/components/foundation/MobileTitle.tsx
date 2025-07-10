import { useRouter } from "@tanstack/react-router";

import { IconButton, IconChevronDown, twMerge } from "@left-curve/applets-kit";

import type React from "react";

type MobileTitleProps = {
  title: string;
  className?: string;
};

export const MobileTitle: React.FC<MobileTitleProps> = ({ title, className }) => {
  const { history } = useRouter();
  return (
    <div className={twMerge("flex gap-2 items-center lg:hidden self-start", className)}>
      <IconButton variant="link" onClick={() => history.go(-1)}>
        <IconChevronDown className="rotate-90" />
      </IconButton>

      <h2 className="h3-bold text-primary-900">{title}</h2>
    </div>
  );
};
