import path from "node:path";
import type { Rspack } from "@rsbuild/core";

const stableR2AssetTypes = {
  font: new Set([".eot", ".otf", ".ttf", ".woff", ".woff2"]),
  image: new Set([
    ".apng",
    ".avif",
    ".bmp",
    ".cur",
    ".gif",
    ".ico",
    ".jfif",
    ".jpg",
    ".jpeg",
    ".pjp",
    ".pjpeg",
    ".png",
    ".tif",
    ".tiff",
    ".webp",
  ]),
  svg: new Set([".svg"]),
} as const;

type StableR2AssetType = keyof typeof stableR2AssetTypes;

type AssetPathData = {
  filename?: string;
  module?: {
    matchResource?: string;
    nameForCondition?: () => string | null;
    resource?: string;
  };
};

type AssetInfo = unknown;
type AssetModuleFilename = string | ((pathData: AssetPathData, assetInfo?: AssetInfo) => string);
type AssetGenerator = {
  publicPath?: string | ((pathData: AssetPathData, assetInfo?: AssetInfo) => string);
};

type AssetRule = {
  generator?: AssetGenerator;
  oneOf?: AssetRule[];
  rules?: AssetRule[];
  type?: string;
};

type PortalAssetOptions = {
  portalRoot: string;
  r2AssetsPrefix: string;
  stableR2AssetsPrefix: string;
  useR2Assets: boolean;
  workspaceRoot: string;
};

const getAssetSource = (pathData: AssetPathData) =>
  pathData.filename ||
  pathData.module?.matchResource ||
  pathData.module?.resource ||
  pathData.module?.nameForCondition?.() ||
  "";

const getStableR2AssetType = (pathData: AssetPathData): StableR2AssetType | null => {
  const assetPath = getAssetSource(pathData).split(/[?#]/)[0];
  const extension = path.extname(assetPath).toLowerCase();

  return (
    (Object.entries(stableR2AssetTypes) as [StableR2AssetType, Set<string>][]).find(
      ([, extensions]) => extensions.has(extension),
    )?.[0] || null
  );
};

const getStableR2AssetFilename = (pathData: AssetPathData) => {
  const assetType = getStableR2AssetType(pathData);
  return assetType ? `static/${assetType}/[name].[contenthash:8][ext][query]` : null;
};

const setStableR2AssetRulePublicPath = (
  rules: AssetRule[] | undefined,
  publicPath: NonNullable<AssetGenerator["publicPath"]>,
) => {
  if (!rules) return;

  for (const rule of rules) {
    if (rule.type === "asset" || rule.type === "asset/resource") {
      rule.generator = {
        ...rule.generator,
        publicPath,
      };
    }

    setStableR2AssetRulePublicPath(rule.rules, publicPath);
    setStableR2AssetRulePublicPath(rule.oneOf, publicPath);
  }
};

const getImagePathTransformIncludes = (portalRoot: string, workspaceRoot: string) => [
  path.resolve(portalRoot, "constants.config.ts"),
  path.resolve(portalRoot, "store.config.ts"),
  path.resolve(portalRoot, "src"),
  path.resolve(portalRoot, "node_modules", "@left-curve", "foundation"),
  path.resolve(portalRoot, "node_modules", "@left-curve", "store", "src"),
  path.resolve(workspaceRoot, "ui/foundation"),
  path.resolve(workspaceRoot, "ui/store/src"),
];

const addImagePathTransformRule = (
  config: Rspack.Configuration,
  { portalRoot, workspaceRoot }: Pick<PortalAssetOptions, "portalRoot" | "workspaceRoot">,
) => {
  config.module ??= {};
  config.module.rules ??= [];

  config.module.rules.unshift({
    test: /\.[cm]?[jt]sx?$/,
    include: getImagePathTransformIncludes(portalRoot, workspaceRoot),
    enforce: "pre",
    loader: path.resolve(portalRoot, "scripts/image-path-transform-loader.cjs"),
  });
};

const configureStableR2AssetOutput = (
  config: Rspack.Configuration,
  { r2AssetsPrefix, stableR2AssetsPrefix, useR2Assets }: PortalAssetOptions,
) => {
  config.output ??= {};

  const fallbackAssetModuleFilename = config.output.assetModuleFilename as
    | AssetModuleFilename
    | undefined;

  config.output.assetModuleFilename = ((pathData: AssetPathData, assetInfo?: AssetInfo) => {
    const stableFilename = getStableR2AssetFilename(pathData);
    if (stableFilename) return stableFilename;
    if (typeof fallbackAssetModuleFilename === "function") {
      return fallbackAssetModuleFilename(pathData, assetInfo);
    }
    return fallbackAssetModuleFilename || "static/assets/[name].[contenthash:8][ext][query]";
  }) as NonNullable<NonNullable<Rspack.Configuration["output"]>["assetModuleFilename"]>;

  if (!useR2Assets) return;

  const getStableR2AssetPublicPath = (pathData: AssetPathData) =>
    getStableR2AssetType(pathData) ? stableR2AssetsPrefix : r2AssetsPrefix;

  config.module ??= {};
  config.module.generator ??= {};

  const assetGenerator = (config.module.generator.asset || {}) as AssetGenerator;
  config.module.generator.asset = {
    ...assetGenerator,
    publicPath: getStableR2AssetPublicPath,
  };

  const assetResourceGenerator = (config.module.generator["asset/resource"] ||
    {}) as AssetGenerator;
  config.module.generator["asset/resource"] = {
    ...assetResourceGenerator,
    publicPath: getStableR2AssetPublicPath,
  };

  setStableR2AssetRulePublicPath(
    config.module.rules as AssetRule[] | undefined,
    getStableR2AssetPublicPath,
  );
};

export const configurePortalAssets = (
  config: Rspack.Configuration,
  options: PortalAssetOptions,
) => {
  addImagePathTransformRule(config, options);
  configureStableR2AssetOutput(config, options);
};
