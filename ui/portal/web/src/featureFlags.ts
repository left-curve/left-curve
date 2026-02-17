const FEATURE_IDS = ["points"] as const;

export type FeatureId = (typeof FEATURE_IDS)[number];

const FEATURE_SET = new Set<string>(FEATURE_IDS);

const normalizeFeatureId = (value: string) => value.trim().toLowerCase();

const getDisabledFeatures = (): Set<FeatureId> => {
  const disabledFeatures = window.dango?.disabledFeatures;
  if (!Array.isArray(disabledFeatures)) return new Set<FeatureId>();

  return new Set(
    disabledFeatures
      .map(normalizeFeatureId)
      .filter((feature): feature is FeatureId => FEATURE_SET.has(feature)),
  );
};

export const isFeatureEnabled = (feature: FeatureId): boolean => {
  return !getDisabledFeatures().has(feature);
};
