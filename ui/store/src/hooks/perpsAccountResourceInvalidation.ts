import {
  createLiveResourceInvalidator,
  useLiveResourceInvalidationRevision,
} from "../live/invalidation.js";

export type PerpsAccountResourceInvalidationParameters = {
  perpsContract: string;
  accountAddress?: string;
};

const perpsAccountResourceInvalidator = createLiveResourceInvalidator();

function getPerpsAccountResourceInvalidationKey(
  parameters: PerpsAccountResourceInvalidationParameters,
) {
  const { accountAddress, perpsContract } = parameters;
  if (!accountAddress) return null;
  return `perpsAccount:${perpsContract}:${accountAddress}`;
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
