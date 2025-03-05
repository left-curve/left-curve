import type React from "react";

export const SwapArrowDownIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({
  ...props
}) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="15"
      fill="none"
      viewBox="0 0 20 15"
      {...props}
    >
      <path
        fill="currentColor"
        d="M11.737 14.224c4.133-3.212 6.293-7.614 7.928-12.346.245-.709-.506-1.39-1.212-1.136-1.914.691-5.1 1.615-8.453 1.615-3.354 0-6.54-.924-8.453-1.615C.84.487.089 1.169.335 1.878 1.97 6.61 4.13 11.012 8.263 14.224a2.83 2.83 0 0 0 3.474 0"
      />
    </svg>
  );
};
