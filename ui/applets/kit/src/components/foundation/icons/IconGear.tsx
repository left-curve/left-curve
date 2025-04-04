import type React from "react";

export const IconGear: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48" {...props}>
      <path
        fill="currentColor"
        fillRule="evenodd"
        d="M22.66.855h2.687a5.01 5.01 0 0 1 4.656 3.182l1.231 3.127 3.737 2.157 3.312-.504a5.01 5.01 0 0 1 5.088 2.451l1.341 2.318a5.005 5.005 0 0 1-.422 5.633L42.2 21.835v4.327l2.078 2.616a5.02 5.02 0 0 1 .421 5.633l-1.337 2.318a5.02 5.02 0 0 1-5.088 2.451l-3.315-.504-3.74 2.16-1.225 3.12a5.01 5.01 0 0 1-4.656 3.185h-2.684a5.01 5.01 0 0 1-4.656-3.185l-1.224-3.12-3.741-2.16-3.315.504a5.01 5.01 0 0 1-5.088-2.448L3.29 34.411a5.01 5.01 0 0 1 .422-5.633l2.092-2.616v-4.327l-2.092-2.616a5.01 5.01 0 0 1-.425-5.633l1.337-2.318a5.01 5.01 0 0 1 5.092-2.451l3.305.504 3.75-2.174 1.231-3.11A5.01 5.01 0 0 1 22.66.855m8.183 23.143c0 4.379-2.465 6.844-6.843 6.844s-6.843-2.465-6.843-6.844c0-4.378 2.465-6.843 6.843-6.843s6.843 2.465 6.843 6.843"
        clipRule="evenodd"
      />
    </svg>
  );
};
