import {
  createLiveResourceInvalidator,
  useLiveResourceInvalidationRevision,
} from "../live/invalidation.js";

import type { Config } from "../types/store.js";

export type PerpsAccountResourceInvalidationParameters = {
  chainId: Config["chain"]["id"];
  perpsContract: string;
  accountAddress?: string;
};

const perpsAccountResourceInvalidator = createLiveResourceInvalidator();

function getPerpsAccountResourceInvalidationKey(
  parameters: PerpsAccountResourceInvalidationParameters,
) {
  const { accountAddress, chainId, perpsContract } = parameters;
  if (!accountAddress) return null;
  return `perpsAccount:${chainId}:${perpsContract}:${accountAddress}`;
}

export function invalidatePerpsAccountResources(
  parameters: PerpsAccountResourceInvalidationParameters,
) {
  const key = getPerpsAccountResourceInvalidationKey(parameters);
  if (!key) return;

  perpsAccountResourceInvalidator.invalidate(key);
}

export function usePerpsAccountResourceRevision(
  parameters: PerpsAccountResourceInvalidationParameters,
) {
  return useLiveResourceInvalidationRevision(
    perpsAccountResourceInvalidator,
    getPerpsAccountResourceInvalidationKey(parameters),
  );
}
