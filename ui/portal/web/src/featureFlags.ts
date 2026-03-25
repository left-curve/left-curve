const FEATURE_IDS = [
  "points",
  "referral",
  "stopLoss",
  "triggerOrders",
  "orderTimestamp",
  "perpsOriginalSize",
] as const;

export type FeatureId = (typeof FEATURE_IDS)[number];

const FEATURE_SET = new Set<string>(FEATURE_IDS);

const normalizeFeatureId = (value: string) => value.trim().toLowerCase();

const getEnabledFeatures = (): Set<FeatureId> => {
  const enabledFeatures = window.dango?.enabledFeatures;
  if (!Array.isArray(enabledFeatures)) return new Set<FeatureId>();

  return new Set(
    enabledFeatures
      .map(normalizeFeatureId)
      .filter((feature): feature is FeatureId => FEATURE_SET.has(feature)),
  );
};

export const isFeatureEnabled = (feature: FeatureId): boolean => {
  return getEnabledFeatures().has(feature);
};
