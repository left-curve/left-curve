module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  transform: {
    "^.+\\.ts$": [
      "ts-jest",
      {
        // we must make two changes in tsconfig for jest to work.
        //
        // 1. `verbatimModuleSyntax` to false
        // https://github.com/kulshekhar/ts-jest/issues/4081#issuecomment-1515758013
        //
        // otherwise we get error:
        //
        // > error TS1286: ESM syntax is not allowed in a CommonJS module when 'verbatimModuleSyntax'
        // > is enabled
        //
        // 2. `noEmitOnError` to false
        // https://github.com/kulshekhar/ts-jest/issues/4246#issuecomment-1841354601
        //
        // otherwise we get error:
        //
        // > Unable to process '*.test.ts', please make sure that `outDir` in your tsconfig is neither
        // > `''` or `'.'`. You can also configure Jest config option `transformIgnorePatterns` to
        // > inform `ts-jest` to transform *.test.ts
        //
        // I have zero clue what these errors mean or what the fixes do, but at
        // least everything (build, lint, test, docs) works now.
        //
        // Still an unsolved issue is I can't use `.ts` for jest config. Can't
        // get jest to parse it whatever I try.
        tsconfig: "./tsconfig.test.json",
      },
    ],
  },
};
