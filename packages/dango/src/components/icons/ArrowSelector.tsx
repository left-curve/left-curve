import type React from "react";

export const ArrowSelectorIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({
  ...props
}) => {
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
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
        d="M2.143 7.072c2.21 4.418 4.419 6.856 8.404 9.09a2.976 2.976 0 002.907 0c3.985-2.234 6.194-4.672 8.403-9.09"
      />
    </svg>
  );
};
