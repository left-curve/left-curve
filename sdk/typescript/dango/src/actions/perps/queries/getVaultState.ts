import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, Prettify } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import type {
  PerpsQueryMsg,
  PerpsState,
  PerpsUserStateExtended,
  PerpsVaultState,
} from "@left-curve/types";

export type GetPerpsVaultStateParameters = Prettify<{ height?: number }>;

export type GetPerpsVaultStateReturnType = Promise<PerpsVaultState>;

export async function getPerpsVaultState(
  client: Client,
  parameters?: GetPerpsVaultStateParameters,
): GetPerpsVaultStateReturnType {
  const { height = 0 } = parameters ?? {};

  const { addresses } = await getAppConfig(client);
  const perpsContract = addresses.perps;

  const stateMsg: PerpsQueryMsg = { state: {} };
  const state: PerpsState = await queryWasmSmart(client, {
    contract: perpsContract,
    msg: stateMsg,
    height,
  });

  const userStateMsg: PerpsQueryMsg = {
    userStateExtended: {
      user: perpsContract,
      includeEquity: true,
      includeAvailableMargin: false,
      includeMaintenanceMargin: false,
      includeUnrealizedPnl: false,
      includeUnrealizedFunding: false,
      includeLiquidationPrice: false,
    },
  };
  const vaultUserState: PerpsUserStateExtended | null = await queryWasmSmart(client, {
    contract: perpsContract,
    msg: userStateMsg,
    height,
  });

  return {
    shareSupply: state.vaultShareSupply,
    equity: vaultUserState?.equity ?? "0",
    depositWithdrawalActive: true,
    margin: vaultUserState?.margin ?? "0",
    positions: vaultUserState?.positions ?? {},
    reservedMargin: vaultUserState?.reservedMargin ?? "0",
    openOrderCount: vaultUserState?.openOrderCount ?? 0,
  };
}
