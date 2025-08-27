import type React from "react";

export const IconMarketOrder: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="19"
      fill="none"
      viewBox="0 0 20 19"
      {...props}
    >
      <g clipPath="url(#clip0_9524_42384)">
        <path
          fill="currentColor"
          fillRule="evenodd"
          d="M10.59 1.257a.587.587 0 0 0-.586-.589c-1.825 0-3.3.055-4.826.16a3.23 3.23 0 0 0-3.004 3.02 88 88 0 0 0 0 10.973 3.23 3.23 0 0 0 3.005 3.02c1.525.106 3 .16 4.824.16 1.825 0 3.3-.054 4.825-.16a3.23 3.23 0 0 0 3.006-3.02 87 87 0 0 0 .162-4.322.7.7 0 0 0-.018-.16c-.322-1.303-.964-1.999-1.67-2.398-.838-.474-1.876-.594-2.88-.588-1.525.011-2.837-1.137-2.837-2.744zm7.37 5.824c.007.006.016 0 .016-.008a3.2 3.2 0 0 0-.618-1.821c-1.204-1.641-2.182-2.681-3.776-3.912a3 3 0 0 0-.556-.343c-.38-.181-.767.138-.767.556V4.61c0 .6.475 1.082 1.16 1.078 1.097-.008 2.49.112 3.712.804q.44.249.83.59"
          clipRule="evenodd"
        />
        <path
          stroke="transparent"
          strokeLinecap="round"
          strokeWidth="2.333"
          d="M6.112 9.724h7.778M6.112 5.835h2.333M6.112 13.611h7.778"
        />
      </g>
      <defs>
        <clipPath id="clip0_9524_42384">
          <path fill="#fff" d="M.667.002h18.667v18.667H.667z" />
        </clipPath>
      </defs>
    </svg>
  );
};
