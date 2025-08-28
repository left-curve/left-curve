import type React from "react";

export const IconTimer: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="25"
      fill="none"
      viewBox="0 0 24 25"
      {...props}
    >
      <circle cx="12" cy="14" r="9" stroke="currentColor" strokeWidth="2.5" />
      <path
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="2.5"
        d="M10 2h4M12 2v3M12 14l3-3M18.5 7l.5-.5"
      />
    </svg>
  );
};
