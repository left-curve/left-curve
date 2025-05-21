import type React from "react";

export const IconUserCircle: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      fill="none"
      viewBox="0 0 24 24"
      {...props}
    >
      <rect width="23" height="23" x="0.5" y="0.5" fill="currentColor" rx="11.5" />
      <rect width="23" height="23" x="0.5" y="0.5" stroke="currentColor" rx="11.5" />
      <path
        fill="currentColor"
        fillRule="evenodd"
        d="M16.166 8.899c0 2.735-1.538 4.274-4.274 4.274s-4.274-1.539-4.274-4.274 1.539-4.274 4.274-4.274 4.274 1.538 4.274 4.274M11.892 23.49c3.37 0 8.205 0 8.205-2.286 0-1.705-1.655-3.25-3.13-4.324a8.62 8.62 0 0 0-10.15 0c-1.474 1.074-3.13 2.619-3.13 4.324 0 2.286 4.835 2.286 8.205 2.286"
        clipRule="evenodd"
      />
    </svg>
  );
};
