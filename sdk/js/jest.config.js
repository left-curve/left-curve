/** @type {import("ts-jest").JestConfigWithTsJest} */
export default {
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
