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
  type SubmitConditionalOrderParameters,
  type SubmitConditionalOrderReturnType,
  submitConditionalOrder,
} from "./mutations/submitConditionalOrder.js";

export {
  type CancelConditionalOrderParameters,
  type CancelConditionalOrderReturnType,
  cancelConditionalOrder,
} from "./mutations/cancelConditionalOrder.js";
