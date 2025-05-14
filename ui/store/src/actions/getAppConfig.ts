import { useConfig } from "../hooks/useConfig.js";
import type { Config } from "../types/store.js";

export type GetAppConfigReturnType = ReturnType<ReturnType<typeof useConfig>["getAppConfig"]>;

export type GetAppConfigErrorType = Error;

export async function getAppConfig<config extends Config>(
  config: config,
): Promise<GetAppConfigReturnType> {
  const { getAppConfig } = useConfig({ config });
  return await getAppConfig();
}
