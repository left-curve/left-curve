import type React from "react";

export const CollapseIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      fill="currentColor"
      viewBox="0 0 16 16"
      {...props}
    >
      <path
        fillRule="evenodd"
        d="M1 8a.5.5 0 01.5-.5h13a.5.5 0 010 1h-13A.5.5 0 011 8m7-8a.5.5 0 01.5.5v3.793l1.146-1.147a.5.5 0 01.708.708l-2 2a.5.5 0 01-.708 0l-2-2a.5.5 0 11.708-.708L7.5 4.293V.5A.5.5 0 018 0m-.5 11.707l-1.146 1.147a.5.5 0 01-.708-.708l2-2a.5.5 0 01.708 0l2 2a.5.5 0 01-.708.708L8.5 11.707V15.5a.5.5 0 01-1 0z"
      />
    </svg>
  );
};
