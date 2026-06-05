type ImageModule = string | { default: string };

type ImageContext = {
  keys(): string[];
  (path: string): ImageModule;
};

declare const require: {
  context(request: string, useSubdirectories?: boolean, regExp?: RegExp): ImageContext;
};

const imageContext = require.context(
  "~/images",
  true,
  /\.(apng|avif|bmp|cur|gif|ico|jfif|jpe?g|pjpe?g|png|svg|tiff?|webp)$/i,
);

const toImageUrl = (imageModule: ImageModule) =>
  typeof imageModule === "string" ? imageModule : imageModule.default;

const imageUrlsByPublicPath: Readonly<Record<string, string>> = Object.fromEntries(
  imageContext
    .keys()
    .map((key) => [`/images/${key.replace(/^\.\//, "")}`, toImageUrl(imageContext(key))]),
);

const missingManifestEntries = new Set<string>();

function shouldWarnMissingManifestEntry() {
  return import.meta.env.CONFIG_ENVIRONMENT !== "prod";
}

function reportMissingManifestEntry(src: string) {
  if (!shouldWarnMissingManifestEntry() || missingManifestEntries.has(src)) return;

  missingManifestEntries.add(src);
  console.warn(`[imageUrl] Missing bundled image entry for "${src}".`);
}

export function imageUrl(src: string): string {
  if (!src.startsWith("/images/")) return src;

  const resolved = imageUrlsByPublicPath[src];
  if (resolved) return resolved;

  reportMissingManifestEntry(src);
  return src;
}
