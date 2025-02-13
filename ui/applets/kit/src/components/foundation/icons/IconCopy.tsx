import type React from "react";

export const IconCopy: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
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
        d="M9.413 4.25a2.15 2.15 0 0 0-1.528.633 1.25 1.25 0 0 1-1.77-1.766 4.65 4.65 0 0 1 3.3-1.367H18A4.25 4.25 0 0 1 22.25 6v8.584l-1.25.002h1.25a4.65 4.65 0 0 1-1.367 3.299 1.25 1.25 0 1 1-1.766-1.77 2.15 2.15 0 0 0 .633-1.528V6A1.75 1.75 0 0 0 18 4.25zM6 8.25A1.75 1.75 0 0 0 4.25 10v8c0 .966.784 1.75 1.75 1.75h8A1.75 1.75 0 0 0 15.75 18v-8A1.75 1.75 0 0 0 14 8.25zM1.75 10A4.25 4.25 0 0 1 6 5.75h8A4.25 4.25 0 0 1 18.25 10v8A4.25 4.25 0 0 1 14 22.25H6A4.25 4.25 0 0 1 1.75 18z"
        clipRule="evenodd"
      />
    </svg>
  );
};
