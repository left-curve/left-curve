import type React from "react";

export const ProfileIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="44"
      height="44"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.5"
      viewBox="0 0 24 24"
      {...props}
    >
      <path stroke="none" d="M0 0h24v24H0z" />
      <path d="M3 12a9 9 0 1018 0 9 9 0 10-18 0" />
      <path d="M9 10a3 3 0 106 0 3 3 0 10-6 0M6.168 18.849A4 4 0 0110 16h4a4 4 0 013.834 2.855" />
    </svg>
  );
};
