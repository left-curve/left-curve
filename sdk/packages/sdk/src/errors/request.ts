import { BaseError } from "./base";

export type HttpRequestErrorType = HttpRequestError & {
  name: "HttpRequestError";
};
export class HttpRequestError extends BaseError {
  body?: { [x: string]: unknown } | { [y: string]: unknown }[] | undefined;
  headers?: Headers | undefined;
  status?: number | undefined;
  url: string;

  constructor({
    body,
    cause,
    details,
    headers,
    status,
    url,
  }: {
    body?: { [x: string]: unknown } | { [y: string]: unknown }[] | undefined;
    cause?: Error | undefined;
    details?: string | undefined;
    headers?: Headers | undefined;
    status?: number | undefined;
    url: string;
  }) {
    super("HTTP request failed.", {
      cause,
      details,
      metaMessages: [
        status && `Status: ${status}`,
        `URL: ${url}`,
        body && `Request body: ${JSON.stringify(body)}`,
      ].filter(Boolean) as string[],
      name: "HttpRequestError",
    });
    this.body = body;
    this.headers = headers;
    this.status = status;
    this.url = url;
  }
}
