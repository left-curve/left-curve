module.exports = {
  entryPoints: ["./src/index.ts"],
  out: "./docs",
  exclude: "**/*.test.ts",
  name: "Grug Documentation",
  readme: "README.md",
  excludeExternals: true,
  excludePrivate: true,
};
