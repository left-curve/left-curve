import { serializeJson } from "@leftcurve/encoding";
import { BaseError } from "./base";

export type TimeoutErrorType = TimeoutError & {
  name: "TimeoutError";
};

export class TimeoutError extends BaseError {
  constructor({
    body,
    url,
  }: {
    body: { [x: string]: unknown } | { [y: string]: unknown }[];
    url: string;
  }) {
    super("The request took too long to respond.", {
      details: "The request timed out.",
      metaMessages: [`URL: ${url}`, `Request body: ${serializeJson(body)}`],
      name: "TimeoutError",
    });
  }
}
