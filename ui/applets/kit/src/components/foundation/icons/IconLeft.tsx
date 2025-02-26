import type React from "react";

export const IconLeft: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
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
        d="M3.303 11.924c0-.966.783-1.75 1.75-1.75l14.895-.002a1.75 1.75 0 1 1 0 3.5l-14.895.002a1.75 1.75 0 0 1-1.75-1.75"
        clipRule="evenodd"
      />
      <path
        fill="currentColor"
        fillRule="evenodd"
        d="M9.622 19.429c-2.75-1.374-5.434-4.06-6.808-6.808a1.75 1.75 0 0 1 0-1.565c1.374-2.75 4.059-5.434 6.808-6.808a1.75 1.75 0 1 1 1.565 3.13c-1.811.906-3.67 2.651-4.802 4.46 1.131 1.81 2.991 3.555 4.802 4.46a1.75 1.75 0 0 1-1.565 3.13"
        clipRule="evenodd"
      />
    </svg>
  );
};
