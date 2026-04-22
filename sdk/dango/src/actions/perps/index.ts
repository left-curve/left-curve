export {
  type PerpsQueryActions,
  perpsQueryActions,
  type PerpsMutationActions,
  perpsMutationActions,
} from "./perpsActions.js";

export {
  type GetPerpsUserStateParameters,
  type GetPerpsUserStateReturnType,
  getPerpsUserState,
} from "./queries/getUserState.js";

export {
  type GetPerpsOrdersByUserParameters,
  type GetPerpsOrdersByUserReturnType,
  getPerpsOrdersByUser,
} from "./queries/getOrdersByUser.js";

export {
  type GetPerpsLiquidityDepthParameters,
  type GetPerpsLiquidityDepthReturnType,
  getPerpsLiquidityDepth,
} from "./queries/getLiquidityDepth.js";

export {
  type GetPerpsPairParamParameters,
  type GetPerpsPairParamReturnType,
  getPerpsPairParam,
} from "./queries/getPairParam.js";

export {
  type GetPerpsPairParamsParameters,
  type GetPerpsPairParamsReturnType,
  getPerpsPairParams,
} from "./queries/getPairParams.js";

export {
  type GetPerpsParamParameters,
  type GetPerpsParamReturnType,
  getPerpsParam,
} from "./queries/getParam.js";

export {
  type QueryPerpsCandlesParameters,
  type QueryPerpsCandlesReturnType,
  queryPerpsCandles,
} from "./queries/perpsCandles.js";

export {
  type QueryPerpsEventsParameters,
  type QueryPerpsEventsReturnType,
  queryPerpsEvents,
} from "./queries/perpsEvents.js";

export {
  type GetPerpsPairStatsParameters,
  type GetPerpsPairStatsReturnType,
  getPerpsPairStats,
} from "./queries/getPerpsPairStats.js";

export {
  type GetAllPerpsPairStatsReturnType,
  getAllPerpsPairStats,
} from "./queries/getAllPerpsPairStats.js";

export {
  type GetPerpsPairStateParameters,
  type GetPerpsPairStateReturnType,
  getPerpsPairState,
} from "./queries/getPerpsPairState.js";

export {
  type GetPerpsStateParameters,
  type GetPerpsStateReturnType,
  getPerpsState,
} from "./queries/getPerpsState.js";

export {
  type DepositMarginParameters,
  type DepositMarginReturnType,
  depositMargin,
} from "./mutations/depositMargin.js";

export {
  type WithdrawMarginParameters,
  type WithdrawMarginReturnType,
  withdrawMargin,
} from "./mutations/withdrawMargin.js";

export {
  type SubmitPerpsOrderParameters,
  type SubmitPerpsOrderReturnType,
  submitPerpsOrder,
} from "./mutations/submitOrder.js";

export {
  type CancelPerpsOrderParameters,
  type CancelPerpsOrderReturnType,
  cancelPerpsOrder,
} from "./mutations/cancelOrder.js";

export {
  type SetReferralParameters,
  type SetReferralReturnType,
  setReferral,
} from "./mutations/setReferral.js";

export {
  type SetFeeShareRatioParameters,
  type SetFeeShareRatioReturnType,
  setFeeShareRatio,
} from "./mutations/setFeeShareRatio.js";

export {
  type GetPerpsVaultStateParameters,
  type GetPerpsVaultStateReturnType,
  getPerpsVaultState,
} from "./queries/getVaultState.js";

export {
  type GetFeeRateOverrideParameters,
  type GetFeeRateOverrideReturnType,
  getFeeRateOverride,
} from "./queries/getFeeRateOverride.js";

export type { FeeRateOverride } from "../../types/perps.js";

export {
  type VaultAddLiquidityParameters,
  type VaultAddLiquidityReturnType,
  vaultAddLiquidity,
} from "./mutations/vaultAddLiquidity.js";

export {
  type VaultRemoveLiquidityParameters,
  type VaultRemoveLiquidityReturnType,
  vaultRemoveLiquidity,
} from "./mutations/vaultRemoveLiquidity.js";

export {
  type SubmitConditionalOrderParameters,
  type SubmitConditionalOrderReturnType,
  submitConditionalOrder,
} from "./mutations/submitConditionalOrder.js";

export {
  type SubmitConditionalOrderInput,
  type SubmitConditionalOrdersParameters,
  type SubmitConditionalOrdersReturnType,
  submitConditionalOrders,
} from "./mutations/submitConditionalOrders.js";

export {
  type CancelConditionalOrderParameters,
  type CancelConditionalOrderReturnType,
  cancelConditionalOrder,
} from "./mutations/cancelConditionalOrder.js";
