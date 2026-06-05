const parser = require("@babel/parser");
const traverse = require("@babel/traverse").default;
const MagicString = require("magic-string");

const IMAGE_PATH_PREFIX = "/images/";
const IMAGE_REQUEST_PREFIX = "~/images";
const IMAGE_EXTENSION_PATTERN =
  /\.(apng|avif|bmp|cur|gif|ico|jfif|jpe?g|pjpe?g|png|svg|tiff?|webp)([?#].*)?$/i;

const PARSER_PLUGINS = [
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

const isImagePath = (value) =>
  typeof value === "string" &&
  value.startsWith(IMAGE_PATH_PREFIX) &&
  IMAGE_EXTENSION_PATTERN.test(value);

const getImageExtension = (value) => {
  const match = value.match(IMAGE_EXTENSION_PATTERN);
  return match ? match[1].toLowerCase() : null;
};

const publicPathToRequest = (publicPath) =>
  `${IMAGE_REQUEST_PREFIX}/${publicPath.slice(IMAGE_PATH_PREFIX.length)}`;

const publicDirToRequest = (publicDir) =>
  `${IMAGE_REQUEST_PREFIX}${publicDir.slice("/images".length)}`;

const normalizeRegExpExtension = (extension) => {
  if (extension === "jpg" || extension === "jpeg") return "jpe?g";
  if (extension === "pjp" || extension === "pjpeg") return "pjpe?g";
  if (extension === "tif" || extension === "tiff") return "tiff?";
  return extension.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
};

const shouldSkipStringLiteral = (path) => {
  const parent = path.parent;

  if (!parent) return false;
  if (parent.type === "Directive") return true;

  if (
    (parent.type === "ImportDeclaration" ||
      parent.type === "ExportAllDeclaration" ||
      parent.type === "ExportNamedDeclaration") &&
    parent.source === path.node
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
    parent.key === path.node &&
    !parent.computed
  ) {
    return true;
  }

  return false;
};

const isJsxAttributeValue = (path) =>
  path.parent?.type === "JSXAttribute" && path.parent.value === path.node;

module.exports = function imagePathTransformLoader(source, inputMap) {
  this.cacheable?.();

  if (!source.includes(IMAGE_PATH_PREFIX) && !source.includes("`/images/")) {
    return source;
  }

  const callback = this.async();
  const magic = new MagicString(source);
  const imports = new Map();
  const contexts = new Map();
  let importCount = 0;
  let contextCount = 0;
  let changed = false;

  const getImportIdentifier = (publicPath) => {
    const existing = imports.get(publicPath);
    if (existing) return existing;

    const identifier = `__dangoImageAsset${importCount++}`;
    imports.set(publicPath, identifier);
    return identifier;
  };

  const getContextHelperIdentifier = (publicDir, extension, recursive) => {
    const contextKey = `${publicDir}|${extension}|${recursive}`;
    const existing = contexts.get(contextKey);
    if (existing) return existing.helperIdentifier;

    const index = contextCount++;
    const contextIdentifier = `__dangoImageAssetContext${index}`;
    const helperIdentifier = `__dangoImageAssetUrl${index}`;

    contexts.set(contextKey, {
      contextIdentifier,
      extension,
      helperIdentifier,
      publicDir,
      recursive,
    });

    return helperIdentifier;
  };

  const replaceStaticPath = (path, publicPath) => {
    const identifier = getImportIdentifier(publicPath);

    if (isJsxAttributeValue(path)) {
      magic.overwrite(path.node.start, path.node.end, `{${identifier}}`);
    } else {
      magic.overwrite(path.node.start, path.node.end, identifier);
    }

    changed = true;
  };

  const replaceTemplatePath = (path) => {
    const node = path.node;
    const firstQuasi = node.quasis[0];
    const lastQuasi = node.quasis[node.quasis.length - 1];
    const firstValue = firstQuasi.value.cooked || firstQuasi.value.raw;
    const lastValue = lastQuasi.value.cooked || lastQuasi.value.raw;

    if (!firstValue.startsWith(IMAGE_PATH_PREFIX) || !getImageExtension(lastValue)) {
      return;
    }

    if (node.expressions.length === 0) {
      if (isImagePath(firstValue)) replaceStaticPath(path, firstValue);
      return;
    }

    const lastStaticSlash = firstValue.lastIndexOf("/");
    if (lastStaticSlash < "/images".length) return;

    const publicDir = firstValue.slice(0, lastStaticSlash);
    const firstKeyValue = firstValue.slice(lastStaticSlash + 1);
    const extension = getImageExtension(lastValue);
    if (!extension) return;

    const firstKeyStart = firstQuasi.value.raw.length - firstKeyValue.length;
    const keyQuasis = [firstQuasi.value.raw.slice(firstKeyStart)];

    for (let i = 1; i < node.quasis.length; i++) {
      keyQuasis.push(node.quasis[i].value.raw);
    }

    const recursive = keyQuasis.some((value) => value.includes("/"));
    const helperIdentifier = getContextHelperIdentifier(publicDir, extension, recursive);
    let keyTemplate = "`";

    node.expressions.forEach((expression, index) => {
      keyTemplate += keyQuasis[index];
      keyTemplate += "${";
      keyTemplate += source.slice(expression.start, expression.end);
      keyTemplate += "}";
    });

    keyTemplate += keyQuasis.at(-1);
    keyTemplate += "`";

    magic.overwrite(node.start, node.end, `${helperIdentifier}(${keyTemplate})`);
    changed = true;
  };

  let ast;

  try {
    ast = parser.parse(source, {
      sourceType: "unambiguous",
      plugins: PARSER_PLUGINS,
    });
  } catch (error) {
    callback(error);
    return;
  }

  traverse(ast, {
    StringLiteral(path) {
      if (shouldSkipStringLiteral(path)) return;

      const { value } = path.node;
      if (!isImagePath(value)) return;

      replaceStaticPath(path, value);
    },
    TemplateLiteral(path) {
      replaceTemplatePath(path);
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

  for (const {
    contextIdentifier,
    extension,
    helperIdentifier,
    publicDir,
    recursive,
  } of contexts.values()) {
    header += `const ${contextIdentifier} = require.context(${JSON.stringify(publicDirToRequest(publicDir))}, ${recursive}, /\\.${normalizeRegExpExtension(extension)}$/i);\n`;
    header += `const ${helperIdentifier} = (path) => {\n`;
    header += `  const mod = ${contextIdentifier}(\`./\${path}\`);\n`;
    header += '  return typeof mod === "string" ? mod : mod.default;\n';
    header += "};\n";
  }

  magic.prepend(`${header}\n`);

  callback(null, magic.toString(), magic.generateMap({ hires: true, source: this.resourcePath }));
};
