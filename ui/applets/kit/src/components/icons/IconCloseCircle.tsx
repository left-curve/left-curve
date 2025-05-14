import type React from "react";

export const IconCloseCircle: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      fill="none"
      viewBox="0 0 24 24"
      {...props}
    >
      <circle cx="12" cy="12" r="12" fill="currentColor" />
      <path
        fill="#FFFCF6"
        fillRule="evenodd"
        d="M6.514 6.37a1.5 1.5 0 0 1 2.116.144 91.5 91.5 0 0 0 8.856 8.856 1.5 1.5 0 0 1-1.972 2.26A94.5 94.5 0 0 1 6.37 8.487a1.5 1.5 0 0 1 .144-2.116"
        clipRule="evenodd"
      />
      <path
        fill="#FFFCF6"
        fillRule="evenodd"
        d="M17.486 6.37a1.5 1.5 0 0 1 .145 2.116 94.5 94.5 0 0 1-9.145 9.145 1.5 1.5 0 0 1-1.972-2.261 91.6 91.6 0 0 0 8.856-8.856 1.5 1.5 0 0 1 2.116-.144"
        clipRule="evenodd"
      />
    </svg>
  );
};
