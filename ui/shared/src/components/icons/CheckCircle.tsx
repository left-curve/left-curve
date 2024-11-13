export const CheckCircleIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="20"
      fill="none"
      viewBox="0 0 20 20"
      {...props}
    >
      <g strokeLinecap="round" strokeLinejoin="round" clipPath="url(#clip0_2133_53632)">
        <path d="M10 18.899c5.695 0 8.899-3.204 8.899-8.9 0-5.694-3.204-8.898-8.899-8.898S1.101 4.305 1.101 10 4.305 18.899 10 18.899" />
        <path d="m6.577 10.856 2.49 2.567c1.17-3.365 2.146-4.841 4.356-6.845" />
      </g>
      <defs>
        <clipPath id="clip0_2133_53632">
          <path fill="#fff" d="M0 0h20v20H0z" />
        </clipPath>
      </defs>
    </svg>
  );
};
