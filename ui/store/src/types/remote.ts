import type { Result } from "@left-curve/dango/types";

export type RemoteRequest<Args = any[]> = {
  id: string;
  type: "dango-remote";
  method: string;
  args: Args;
};

export type RemoteResponse<T> = Result<T> & {
  id: string;
  type: "dango-remote";
};
