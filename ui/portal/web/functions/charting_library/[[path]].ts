interface R2HTTPMetadata {
  contentType?: string;
}
interface R2ObjectBody {
  body: ReadableStream;
  httpMetadata?: R2HTTPMetadata;
}
interface R2Bucket {
  get(key: string): Promise<R2ObjectBody | null>;
}

interface CloudflareCache {
  match(request: Request): Promise<Response | undefined>;
  put(request: Request, response: Response): Promise<void>;
}

declare const caches: { default: CloudflareCache };

type PagesFunction<E> = (ctx: {
  request: Request;
  params: Record<string, string | string[]>;
  env: E;
}) => Promise<Response>;

type Env = {
  TV_ASSETS: R2Bucket;
  ASSETS: { fetch: (request: Request) => Promise<Response> };
};

const CONTENT_TYPES: Record<string, string> = {
  js: "application/javascript",
  mjs: "application/javascript",
  css: "text/css",
  html: "text/html",
  json: "application/json",
  wasm: "application/wasm",
  woff: "font/woff",
  woff2: "font/woff2",
  ttf: "font/ttf",
  otf: "font/otf",
  eot: "application/vnd.ms-fontobject",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  svg: "image/svg+xml",
  ico: "image/x-icon",
  map: "application/json",
};

function contentTypeFor(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return CONTENT_TYPES[ext] ?? "application/octet-stream";
}

export const onRequestGet: PagesFunction<Env> = async ({ request, params, env }) => {
  const cache = caches.default;
  const cached = await cache.match(request);
  if (cached) return cached;

  const segments = Array.isArray(params.path) ? params.path : params.path ? [params.path] : [];
  const [version, ...rest] = segments;
  if (!version || rest.length === 0) {
    return env.ASSETS.fetch(request);
  }

  const key = `vendor/tradingview/${version}/charting_library/${rest.join("/")}`;
  const obj = await env.TV_ASSETS.get(key);
  if (!obj) return env.ASSETS.fetch(request);

  const response = new Response(obj.body, {
    headers: {
      "Content-Type": obj.httpMetadata?.contentType ?? contentTypeFor(key),
      "Cache-Control": "public, max-age=31536000, immutable",
    },
  });

  await cache.put(request, response.clone());
  return response;
};
