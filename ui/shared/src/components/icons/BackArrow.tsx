import type React from "react";

export const BackArrowIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      aria-label="Back"
      role="img"
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="2"
      viewBox="0 0 24 24"
      {...props}
    >
      <path stroke="none" d="M0 0h24v24H0z" />
      <path d="M5 12h14M5 12l6 6M5 12l6-6" />
    </svg>
  );
};
