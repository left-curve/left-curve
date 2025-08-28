import type React from "react";

export const IconMobile: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      fill="none"
      viewBox="0 0 24 24"
      {...props}
    >
      <path
        fill="currentColor"
        fillRule="evenodd"
        d="M19.7 5.35c-.2-2.2-1.8-4-3.8-4.2-2.7-.2-5.1-.2-7.8 0-2 .3-3.6 2-3.8 4.2-.1 2.1-.3 4.2-.3 6.5 0 2.2.2 4.4.3 6.4.2 2.2 1.8 4 3.8 4.2 1.3.1 2.6.1 3.9.1s2.6-.1 3.9-.2c2-.2 3.6-1.9 3.8-4.2.1-2 .3-4.2.3-6.4s-.2-4.4-.3-6.4m-12.4.3c.1-.7.5-1.4 1.1-1.4 1.2-.1 2.4-.2 3.6-.2s2.4.1 3.6.2c.5 0 1 .7 1.1 1.4.2 2 .3 4.1.3 6.2 0 1.4-.1 2.9-.2 4.3H7.2c-.1-1.4-.2-2.8-.2-4.3 0-2.1.2-4.2.3-6.2m4.6 13.7c-.6 0-1-.5-1-1 0-.6.5-1 1-1 .6 0 1 .5 1 1 .1.5-.4 1-1 1"
        clipRule="evenodd"
      />
    </svg>
  );
};
