import type React from "react";

export const IconSearch: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="20"
      fill="none"
      viewBox="0 0 20 20"
      {...props}
    >
      <path
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2.92"
        d="m16.666 16.668-2.41-2.41M9.6 15.868c4.011 0 6.267-2.257 6.267-6.268S13.611 3.333 9.6 3.333 3.333 5.589 3.333 9.6s2.256 6.268 6.267 6.268"
      />
    </svg>
  );
};
