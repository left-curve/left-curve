module.exports = {
  parser: "@typescript-eslint/parser",
  parserOptions: {
    ecmaVersion: 2020,
    sourceType: "module",
  },
  plugins: [
    "@stylistic",
    "@typescript-eslint",
  ],
  extends: [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended",
  ],
  rules: {
    "sort-imports": [
      "error",
      {
        "ignoreCase": false,
        "ignoreDeclarationSort": true,
        "ignoreMemberSort": false,
        "allowSeparatedGroups": false,
        "memberSyntaxSortOrder": [
          "none",
          "all",
          "multiple",
          "single",
        ],
      },
    ],
    "@stylistic/comma-dangle": [
      "error",
      "always-multiline",
    ],
    "@stylistic/eol-last": [
      "error",
      "always",
    ],
    "@stylistic/indent": [
      "error",
      2,
    ],
    "@stylistic/max-len": [
      "off", // we don't bother this
      {
        code: 100,
        tabWidth: 2,
        comments: 80,
        ignoreComments: true,
        ignoreTrailingComments: true,
        ignoreUrls: true,
        ignoreStrings: true,
      },
    ],
    "@stylistic/no-tabs": [
      "error",
    ],
    "@stylistic/no-trailing-spaces": [
      "error",
    ],
    "@stylistic/no-multiple-empty-lines": [
      "error",
      {
        max: 1,
      },
    ],
    "@stylistic/quotes": [
      "error",
      "double",
    ],
  },
};
