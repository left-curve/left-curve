import type React from "react";

export const IconTools: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
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
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M1.954 19.633c.083 1.565 1.282 3.033 2.849 3.045h.086c1.566-.012 2.765-1.48 2.848-3.045.026-.481.043-.97.043-1.464 0-.96-.064-1.9-.13-2.808-.057-.781-.652-1.43-1.433-1.492a17 17 0 0 0-2.743 0c-.781.062-1.375.71-1.432 1.492-.066.909-.131 1.847-.131 2.808q.002.742.043 1.464M2.803 4.903c.026 1.128.914 2.073 2.042 2.073s2.016-.945 2.042-2.073c.016-.695.01-1.382-.019-2.083a1.537 1.537 0 0 0-1.444-1.482 11 11 0 0 0-1.158 0A1.537 1.537 0 0 0 2.822 2.82a32 32 0 0 0-.019 2.083"
      />
      <path fill="currentColor" d="M4.845 13.596V7.017z" />
      <path
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
        d="M4.845 13.596V7.017"
      />
      <path
        fill="currentColor"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.5"
        d="M13.596 1.551c0 3.285.9 4.098 2.5 4.098 1.598 0 2.498-.813 2.498-4.098 0-.13.121-.226.246-.19 2.294.658 3.547 2.288 3.547 4.718 0 2.28-1.104 3.856-3.137 4.585a.18.18 0 0 0-.118.18c.062 1.08.117 3.424.117 4.558s-.055 3.504-.117 4.584c-.073 1.286-1.001 2.475-2.282 2.619a7 7 0 0 1-1.51 0c-1.28-.144-2.209-1.333-2.282-2.62a112 112 0 0 1-.117-4.583c0-1.134.055-3.477.117-4.557a.18.18 0 0 0-.118-.18c-2.033-.73-3.136-2.306-3.136-4.586 0-2.43 1.252-4.06 3.546-4.718a.194.194 0 0 1 .246.19"
      />
    </svg>
  );
};
