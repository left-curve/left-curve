import type React from "react";

export const IconCheckedCircle: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({
  ...props
}) => {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <rect width="20" height="20" rx="10" fill="currentColor" />
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M15.407 4.66016C15.7781 5.06922 15.7473 5.70163 15.3382 6.0727C13.8703 7.4043 12.8527 8.52401 12.0079 9.84249C11.1593 11.1671 10.4578 12.735 9.67136 14.9941C9.55729 15.3218 9.28153 15.5669 8.94274 15.6418C8.60396 15.7167 8.25058 15.6106 8.00905 15.3615L4.61511 11.8615C4.23064 11.465 4.24038 10.8319 4.63686 10.4475C5.03335 10.063 5.66644 10.0727 6.05091 10.4692L8.3353 12.825C8.95331 11.2255 9.57725 9.92901 10.3239 8.76357C11.2966 7.24544 12.4501 5.99232 13.9945 4.59138C14.4035 4.22031 15.0359 4.2511 15.407 4.66016Z"
        fill="#FFFCF6"
      />
    </svg>
  );
};
