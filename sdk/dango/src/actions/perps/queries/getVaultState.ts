import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Prettify, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type {
  PerpsQueryMsg,
  PerpsState,
  PerpsUserStateExtended,
  PerpsVaultState,
} from "../../../types/perps.js";

export type GetPerpsVaultStateParameters = Prettify<{ height?: number }>;

export type GetPerpsVaultStateReturnType = Promise<PerpsVaultState>;

export async function getPerpsVaultState<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters?: GetPerpsVaultStateParameters,
): GetPerpsVaultStateReturnType {
  const { height = 0 } = parameters ?? {};

  const action = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await action<AppConfig>({});
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
