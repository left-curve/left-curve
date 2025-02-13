import type React from "react";

export const IconUser: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
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
        d="M16.761 5.884c0 3.127-1.758 4.885-4.884 4.885S6.993 9.01 6.993 5.884 8.75 1 11.877 1s4.884 1.758 4.884 4.884m-4.884 16.677c3.852 0 9.377 0 9.377-2.612a9.85 9.85 0 0 0-18.754 0c0 2.612 5.525 2.612 9.377 2.612"
        clipRule="evenodd"
      />
    </svg>
  );
};
