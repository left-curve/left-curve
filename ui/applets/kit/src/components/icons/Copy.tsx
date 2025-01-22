import type React from "react";

export const CopyIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
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
      <path d="M7 9.667A2.667 2.667 0 019.667 7h8.666A2.667 2.667 0 0121 9.667v8.666A2.667 2.667 0 0118.333 21H9.667A2.667 2.667 0 017 18.333z" />
      <path d="M4.012 16.737A2.005 2.005 0 013 15V5c0-1.1.9-2 2-2h10c.75 0 1.158.385 1.5 1" />
    </svg>
  );
};
