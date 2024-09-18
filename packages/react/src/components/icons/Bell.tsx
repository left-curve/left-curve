import type React from "react";

export const BellIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      fill="currentColor"
      viewBox="0 0 24 24"
      {...props}
    >
      <path fill="none" d="M0 0h24v24H0z" />
      <path d="M14.235 19c.865 0 1.322 1.024.745 1.668A3.992 3.992 0 0112 22a3.992 3.992 0 01-2.98-1.332c-.552-.616-.158-1.579.634-1.661l.11-.006h4.471zM12 2c1.358 0 2.506.903 2.875 2.141l.046.171.008.043a8.013 8.013 0 014.024 6.069l.028.287L19 11v2.931l.021.136a3 3 0 001.143 1.847l.167.117.162.099c.86.487.56 1.766-.377 1.864L20 18H4c-1.028 0-1.387-1.364-.493-1.87a3 3 0 001.472-2.063L5 13.924l.001-2.97A8 8 0 018.822 4.5l.248-.146.01-.043a3.003 3.003 0 012.562-2.29l.182-.017L12 2z" />
    </svg>
  );
};
