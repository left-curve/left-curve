import { parse, type ParserPlugin } from "@babel/parser";
import traverse, { type NodePath } from "@babel/traverse";
import MagicString from "magic-string";

const IMAGE_PATH_PREFIX = "/images/";
const IMAGE_REQUEST_PREFIX = "~/images";
const IMAGE_EXTENSION_PATTERN =
  /\.(apng|avif|bmp|cur|gif|ico|jfif|jpe?g|pjpe?g|png|svg|tiff?|webp)([?#].*)?$/i;

const PARSER_PLUGINS: ParserPlugin[] = [
  "jsx",
  "typescript",
  "importAttributes",
  "decorators-legacy",
  "classProperties",
  "objectRestSpread",
  "dynamicImport",
  "topLevelAwait",
  "importMeta",
];

const traverseAst = ((traverse as unknown as { default?: typeof traverse }).default ??
  traverse) as typeof traverse;

type LoaderCallback = (error: Error | null, source?: string, inputMap?: unknown) => void;

type LoaderContext = {
  async: () => LoaderCallback;
  cacheable?: () => void;
  emitError?: (error: Error) => void;
  emitWarning?: (warning: Error) => void;
  resourcePath: string;
};

type ImageAssetContext = {
  contextIdentifier: string;
  contextRegExpSource: string;
  helperIdentifier: string;
  publicDir: string;
  recursive: boolean;
};

type SourceNode = {
  end?: number | null;
  start?: number | null;
  type?: string;
};

type ImagePathNodePath<TNode extends SourceNode = SourceNode> = NodePath & {
  node: TNode;
  parent?: SourceNode | null;
};

type StringLiteralNode = SourceNode & {
  value: string;
};

type TemplateElementNode = {
  value: {
    cooked?: string | null;
    raw: string;
  };
};

type TemplateLiteralNode = SourceNode & {
  expressions: SourceNode[];
  quasis: TemplateElementNode[];
};

const isImagePath = (value: unknown): value is string =>
  typeof value === "string" &&
  value.startsWith(IMAGE_PATH_PREFIX) &&
  IMAGE_EXTENSION_PATTERN.test(value);

const getImageExtension = (value: string) => {
  const match = value.match(IMAGE_EXTENSION_PATTERN);
  return match ? match[1].toLowerCase() : null;
};

const publicPathToRequest = (publicPath: string) =>
  `${IMAGE_REQUEST_PREFIX}/${publicPath.slice(IMAGE_PATH_PREFIX.length)}`;

const publicDirToRequest = (publicDir: string) =>
  `${IMAGE_REQUEST_PREFIX}${publicDir.slice("/images".length)}`;

const escapeRegExp = (value: string) =>
  value.replace(/[|\\{}()[\]^$+*?.]/g, "\\$&").replace(/\//g, "\\/");

const normalizeRegExpExtension = (extension: string) => {
  if (extension === "jpg" || extension === "jpeg") return "jpe?g";
  if (extension === "pjp" || extension === "pjpeg") return "pjpe?g";
  if (extension === "tif" || extension === "tiff") return "tiff?";
  return escapeRegExp(extension);
};

const getContextRegExpSource = (keyQuasis: string[], extension: string) => {
  const staticSource = keyQuasis.map(escapeRegExp).join(".*");
  if (staticSource) return `^\\.\\/${staticSource}$`;

  return `\\.${normalizeRegExpExtension(extension)}$`;
};

const getRange = (node: SourceNode) => {
  if (typeof node.start !== "number" || typeof node.end !== "number") return null;
  return [node.start, node.end] as const;
};

const shouldSkipStringLiteral = (path: ImagePathNodePath<StringLiteralNode>) => {
  const parent = path.parent;
  const parentRecord = parent as unknown as Record<string, unknown> | null | undefined;

  if (!parent) return false;
  if (parent.type === "Directive") return true;
  if (parent.type.startsWith("TS")) return true;

  if (
    (parent.type === "ImportDeclaration" ||
      parent.type === "ExportAllDeclaration" ||
      parent.type === "ExportNamedDeclaration") &&
    parentRecord?.source === path.node
  ) {
    return true;
  }

  if (
    (parent.type === "ObjectProperty" ||
      parent.type === "ObjectMethod" ||
      parent.type === "ClassProperty" ||
      parent.type === "ClassMethod" ||
      parent.type === "MemberExpression" ||
      parent.type === "OptionalMemberExpression") &&
    parentRecord?.key === path.node &&
    !parentRecord.computed
  ) {
    return true;
  }

  return false;
};

const isJsxAttributeValue = (path: ImagePathNodePath) => {
  const parentRecord = path.parent as unknown as Record<string, unknown> | null | undefined;
  return path.parent?.type === "JSXAttribute" && parentRecord?.value === path.node;
};

export default function imagePathTransformLoader(
  this: LoaderContext,
  source: string,
  inputMap: unknown,
) {
  this.cacheable?.();

  if (!source.includes(IMAGE_PATH_PREFIX) && !source.includes("`/images/")) {
    return source;
  }

  const callback = this.async();
  const magic = new MagicString(source);
  const imports = new Map<string, string>();
  const contexts = new Map<string, ImageAssetContext>();
  let importCount = 0;
  let contextCount = 0;
  let changed = false;

  const getImportIdentifier = (publicPath: string) => {
    const existing = imports.get(publicPath);
    if (existing) return existing;

    const identifier = `__dangoImageAsset${importCount++}`;
    imports.set(publicPath, identifier);
    return identifier;
  };

  const emitTemplateWarning = (node: SourceNode, message: string) => {
    const range = getRange(node);
    const position = range ? ` at offset ${range[0]}` : "";
    const warning = new Error(`${message} in ${this.resourcePath}${position}`);
    this.emitWarning?.(warning);
    if (process.env.CI === "true") this.emitError?.(warning);
  };

  const getContextHelperIdentifier = (
    publicDir: string,
    contextRegExpSource: string,
    recursive: boolean,
  ) => {
    const contextKey = `${publicDir}|${contextRegExpSource}|${recursive}`;
    const existing = contexts.get(contextKey);
    if (existing) return existing.helperIdentifier;

    const index = contextCount++;
    const contextIdentifier = `__dangoImageAssetContext${index}`;
    const helperIdentifier = `__dangoImageAssetUrl${index}`;

    contexts.set(contextKey, {
      contextRegExpSource,
      contextIdentifier,
      helperIdentifier,
      publicDir,
      recursive,
    });

    return helperIdentifier;
  };

  const replaceStaticPath = (path: ImagePathNodePath, publicPath: string) => {
    const range = getRange(path.node);
    if (!range) return;

    const identifier = getImportIdentifier(publicPath);

    if (isJsxAttributeValue(path)) {
      magic.overwrite(range[0], range[1], `{${identifier}}`);
    } else {
      magic.overwrite(range[0], range[1], identifier);
    }

    changed = true;
  };

  const replaceTemplatePath = (path: ImagePathNodePath<TemplateLiteralNode>) => {
    const node = path.node;
    const range = getRange(node);
    if (!range) return;

    const firstQuasi = node.quasis[0];
    const lastQuasi = node.quasis[node.quasis.length - 1];
    const firstValue = firstQuasi.value.cooked || firstQuasi.value.raw;
    const lastValue = lastQuasi.value.cooked || lastQuasi.value.raw;

    if (!firstValue.startsWith(IMAGE_PATH_PREFIX)) {
      return;
    }

    if (node.expressions.length === 0) {
      if (isImagePath(firstValue)) replaceStaticPath(path, firstValue);
      return;
    }

    const extension = getImageExtension(lastValue);
    if (!extension) {
      emitTemplateWarning(node, "Dynamic image template must end with a supported image extension");
      return;
    }

    const lastStaticSlash = firstValue.lastIndexOf("/");
    if (lastStaticSlash < "/images".length) {
      emitTemplateWarning(node, "Dynamic image template must include a static image directory");
      return;
    }

    const publicDir = firstValue.slice(0, lastStaticSlash);
    const firstKeyValue = firstValue.slice(lastStaticSlash + 1);
    if (publicDir === "/images" && firstKeyValue === "") {
      emitTemplateWarning(
        node,
        "Root image templates must include a static filename prefix before interpolation",
      );
      return;
    }

    const firstKeyStart = firstQuasi.value.raw.length - firstKeyValue.length;
    const keyQuasis = [firstQuasi.value.raw.slice(firstKeyStart)];

    for (let i = 1; i < node.quasis.length; i++) {
      keyQuasis.push(node.quasis[i].value.raw);
    }

    const recursive = keyQuasis.some((value) => value.includes("/"));
    const contextRegExpSource = getContextRegExpSource(keyQuasis, extension);
    const helperIdentifier = getContextHelperIdentifier(publicDir, contextRegExpSource, recursive);
    let keyTemplate = "`";
    let canBuildKeyTemplate = true;

    node.expressions.forEach((expression, index) => {
      const expressionRange = getRange(expression);
      if (!expressionRange) {
        emitTemplateWarning(node, "Dynamic image template expression is missing source range data");
        canBuildKeyTemplate = false;
        return;
      }

      keyTemplate += keyQuasis[index];
      keyTemplate += "${";
      keyTemplate += source.slice(expressionRange[0], expressionRange[1]);
      keyTemplate += "}";
    });

    if (!canBuildKeyTemplate) return;

    keyTemplate += keyQuasis.at(-1);
    keyTemplate += "`";

    magic.overwrite(range[0], range[1], `${helperIdentifier}(${keyTemplate})`);
    changed = true;
  };

  let ast: ReturnType<typeof parse>;

  try {
    ast = parse(source, {
      sourceType: "unambiguous",
      plugins: PARSER_PLUGINS,
    });
  } catch (error) {
    callback(error instanceof Error ? error : new Error(String(error)));
    return;
  }

  traverseAst(ast, {
    StringLiteral(path) {
      const stringPath = path as ImagePathNodePath<StringLiteralNode>;
      if (shouldSkipStringLiteral(stringPath)) return;

      const { value } = stringPath.node;
      if (!isImagePath(value)) return;

      replaceStaticPath(stringPath, value);
    },
    TemplateLiteral(path) {
      replaceTemplatePath(path as ImagePathNodePath<TemplateLiteralNode>);
    },
  });

  if (!changed) {
    callback(null, source, inputMap);
    return;
  }

  let header = "";

  for (const [publicPath, identifier] of imports) {
    header += `import ${identifier} from ${JSON.stringify(publicPathToRequest(publicPath))};\n`;
  }

  if (contexts.size > 0) {
    header += 'import * as __dangoImageAssetSentry from "@sentry/react";\n';
  }

  for (const {
    contextRegExpSource,
    contextIdentifier,
    helperIdentifier,
    publicDir,
    recursive,
  } of contexts.values()) {
    header += `const ${contextIdentifier} = require.context(${JSON.stringify(publicDirToRequest(publicDir))}, ${recursive}, /${contextRegExpSource}/i);\n`;
    header += `const ${helperIdentifier} = (path) => {\n`;
    header += `  const fallbackPath = ${JSON.stringify(`${publicDir}/`)} + path;\n`;
    header += "  try {\n";
    header += `    const mod = ${contextIdentifier}(\`./\${path}\`);\n`;
    header += '    return typeof mod === "string" ? mod : mod.default;\n';
    header += "  } catch (error) {\n";
    header +=
      '    __dangoImageAssetSentry.captureException(error, { extra: { imagePath: fallbackPath }, tags: { source: "image-path-loader" } });\n';
    header += "    return fallbackPath;\n";
    header += "  }\n";
    header += "};\n";
  }

  magic.prepend(`${header}\n`);

  callback(null, magic.toString(), magic.generateMap({ hires: true, source: this.resourcePath }));
}
