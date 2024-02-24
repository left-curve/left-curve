module.exports = {
  entryPoints: ["./src/index.ts"],
  out: "./docs",
  exclude: "**/*.test.ts",
  name: "Cw.js Documentation",
  readme: "README.md",
  excludeExternals: true,
  excludePrivate: true,
};
