import type { Config } from "jest";

const config: Config = {
  roots: ["<rootDir>/src"],
  preset: "ts-jest",
  testEnvironment: "node",
  transform: {
    ".ts": [
      "ts-jest",
      {
        useESM: true,
        tsconfig: {
          // https://github.com/kulshekhar/ts-jest/issues/4081
          verbatimModuleSyntax: false,
        }
      }
    ],
  },
};

export default config;
