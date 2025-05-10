import type React from "react";

export const IconArrowDown: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
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
        d="M12.25 20.699a1.75 1.75 0 0 1-1.75-1.75l-.003-14.895a1.75 1.75 0 0 1 3.5 0L14 18.948a1.75 1.75 0 0 1-1.75 1.75"
        clipRule="evenodd"
      />
      <path
        fill="currentColor"
        fillRule="evenodd"
        d="M19.754 14.38c-1.374 2.75-4.059 5.434-6.808 6.808a1.75 1.75 0 0 1-1.565 0c-2.749-1.374-5.434-4.059-6.808-6.808a1.75 1.75 0 0 1 3.13-1.565c.906 1.81 2.652 3.67 4.46 4.802 1.81-1.131 3.555-2.991 4.46-4.802a1.75 1.75 0 1 1 3.131 1.565"
        clipRule="evenodd"
      />
    </svg>
  );
};
