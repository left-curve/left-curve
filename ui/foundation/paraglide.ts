import { compile } from "@inlang/paraglide-js";

(() => {
  console.log("Compiling Paraglide project...");
  compile({
    project: "./project.inlang",
    outdir: "paraglide",
    emitGitIgnore: false,
    emitPrettierIgnore: false,
    includeEslintDisableComment: false,
    strategy: ["localStorage", "preferredLanguage", "baseLocale"],
    localStorageKey: "dango.locale",
  });
})();
