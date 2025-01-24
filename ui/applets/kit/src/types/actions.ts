export const Actions = {} as const;

export type Action = (typeof Actions)[keyof typeof Actions];

export type ActionRequest<payload extends Record<string, any> = Record<string, any>> = {
  id: string;
  action: Action;
  version: number;
  payload: payload;
};

export type ActionResponse<payload = unknown> = {
  id: string;
  status: "success" | "error";
  payload: payload;
};
