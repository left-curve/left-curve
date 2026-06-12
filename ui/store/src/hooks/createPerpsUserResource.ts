import { createLiveResource } from "../live/createLiveResource.js";
import { equalLiveResourcePayload } from "../live/equality.js";
import { handlePerpsUserStateError } from "./perpsUserStateErrors.js";

import { camelCaseJsonDeserialization } from "@left-curve/encoding";

import type { PublicClient } from "@left-curve/sdk";
import type { QueryRequest, QueryResponse } from "@left-curve/types";
import type { LiveResourceSnapshot } from "../live/types.js";
import type { Config } from "../types/store.js";

export type PerpsUserResourceParams = {
  accountAddress: string;
  perpsContract: string;
  publicClient: PublicClient;
  subscriptions: Config["subscriptions"];
};

type CreatePerpsUserResourceParameters<
  Response,
  Snapshot extends LiveResourceSnapshot & { lastUpdatedBlockHeight: number },
> = {
  name: string;
  initialSnapshot: Snapshot;
  payloadKeys: readonly (keyof Snapshot)[];
  interval: number;
  httpInterval: number;
  buildRequest: (
    parameters: Pick<PerpsUserResourceParams, "accountAddress" | "perpsContract">,
  ) => QueryRequest;
  mapResponse: (response: Response | null, blockHeight: number) => Snapshot;
};

function getWasmSmartResponse<Response>(response: QueryResponse) {
  if (!("wasmSmart" in response)) {
    throw new Error(`expecting wasm smart response, got ${JSON.stringify(response)}`);
  }

  return response.wasmSmart as Response | null;
}

export function createPerpsUserResource<
  Response,
  Snapshot extends LiveResourceSnapshot & { lastUpdatedBlockHeight: number },
>({
  name,
  initialSnapshot,
  payloadKeys,
  interval,
  httpInterval,
  buildRequest,
  mapResponse,
}: CreatePerpsUserResourceParameters<Response, Snapshot>) {
  return createLiveResource<PerpsUserResourceParams, Snapshot>({
    name,
    getKey: ({ perpsContract, accountAddress }) => `${name}:${perpsContract}:${accountAddress}`,
    getInitialSnapshot: () => initialSnapshot,
    equal: (previous, next) => equalLiveResourcePayload(previous, next, payloadKeys),
    start: ({ accountAddress, perpsContract, publicClient, subscriptions }, { emit, error }) => {
      const request = buildRequest({ accountAddress, perpsContract });
      let stopped = false;
      let receivedSubscriptionEvent = false;
      let lastUpdatedBlockHeight = 0;

      const emitResponse = (
        response: Response | null,
        blockHeight: number,
        options?: { version: number },
      ) => emit(mapResponse(response, blockHeight), options);

      const handleError = (caught: unknown) => {
        handlePerpsUserStateError(caught, {
          onNotFound: () => emitResponse(null, lastUpdatedBlockHeight),
          onError: error,
        });
      };

      void publicClient
        .queryApp({ query: request })
        .then((response) => {
          if (stopped || receivedSubscriptionEvent) return;
          emitResponse(getWasmSmartResponse<Response>(response), 0);
        })
        .catch((caught) => {
          if (!stopped && !receivedSubscriptionEvent) handleError(caught);
        });

      const unsubscribe = subscriptions.subscribe("queryApp", {
        params: {
          interval,
          httpInterval,
          request,
        },
        listener: (event) => {
          type Event = {
            response: { wasmSmart: Response | null };
            blockHeight: number;
          };
          const { response, blockHeight } = camelCaseJsonDeserialization<Event>(event);
          receivedSubscriptionEvent = true;
          lastUpdatedBlockHeight = blockHeight;
          emitResponse(response.wasmSmart, blockHeight, { version: blockHeight });
        },
        onError: handleError,
      });

      return () => {
        stopped = true;
        unsubscribe();
      };
    },
  });
}
