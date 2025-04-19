import type React from "react";

export const IconChevronDown: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
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
        d="M11.33 13.958a.37.37 0 0 0 .34 0c2.135-1.156 3.276-2.357 4.497-4.719a1.5 1.5 0 1 1 2.665 1.379c-1.468 2.839-3.016 4.506-5.733 5.978a3.37 3.37 0 0 1-3.199 0c-2.716-1.472-4.264-3.14-5.733-5.978a1.5 1.5 0 1 1 2.665-1.379c1.221 2.361 2.363 3.563 4.497 4.719m-.715 1.319.714-1.319z"
        clipRule="evenodd"
      />
    </svg>
  );
};
