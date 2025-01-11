import type { JsonValue } from "./encoding.js";
import type { Message } from "./tx.js";

export type SimulateRequest<Metadata = JsonValue> = {
  sender: string;
  msgs: Message[];
  data: Metadata;
};

export type SimulateResponse = {
  gasLimit: number;
  gasUsed: number;
};
