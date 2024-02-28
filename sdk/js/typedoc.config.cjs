module.exports = {
  entryPoints: ["./src/index.ts"],
  out: "./docs",
  exclude: "**/*.test.ts",
  name: "CW.js Documentation",
  readme: "README.md",
  excludeExternals: true,
  excludePrivate: true,
};
