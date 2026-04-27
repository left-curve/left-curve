use {
    dango_types::{
        config::AppConfig,
        perps::{self, SubmitOrCancelOrderRequest, TraderMsg},
    },
    grug::{Addr, Inner, JsonDeExt, Message, QuerierExt, QuerierWrapper, StdError, Tx},
    prost::bytes::Bytes,
};

/// `ProposalPreparer` implementation that promotes "maker-priority"
/// transactions to the front of the block.
///
/// A transaction qualifies as priority if every one of its messages is a
/// perps trade message that is either a cancel or a post-only submit.
/// Inspired by Hyperliquid's protection of market makers from toxic flow:
/// <https://hyperliquid.medium.com/latency-and-transaction-ordering-on-hyperliquid-cf28df3648eb>
#[derive(Default, Clone, Copy)]
pub struct MakerPriorityHandler;

impl grug_app::ProposalPreparer for MakerPriorityHandler {
    type Error = StdError;

    fn prepare_proposal(
        &self,
        querier: QuerierWrapper,
        txs: Vec<Bytes>,
        _max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        let cfg: AppConfig = querier.query_app_config()?;
        Ok(promote_priority_txs(txs, &cfg.addresses.perps))
    }
}

/// Move all priority transactions to the front of the vector, preserving
/// relative order within each group.
fn promote_priority_txs(mut txs: Vec<Bytes>, perps: &Addr) -> Vec<Bytes> {
    let (priority, other): (Vec<_>, Vec<_>) = txs
        .drain(..)
        .partition(|raw| is_priority_tx(raw.as_ref(), perps));

    txs.extend(priority);
    txs.extend(other);
    txs
}

/// Returns `true` iff every message in the transaction is a perps trade
/// message that is either a cancel or a post-only submit.
///
/// Malformed bytes, non-perps targets, non-trade execute messages, and any
/// disqualifying message all return `false`.
pub fn is_priority_tx(raw_tx: &[u8], perps: &Addr) -> bool {
    let Ok(tx) = raw_tx.deserialize_json::<Tx>() else {
        return false;
    };

    for msg in tx.msgs.into_inner() {
        let Message::Execute(exec) = msg else {
            return false;
        };

        if exec.contract != *perps {
            return false;
        }

        let Ok(execute_msg) = exec.msg.deserialize_json::<perps::ExecuteMsg>() else {
            return false;
        };

        let perps::ExecuteMsg::Trade(trader_msg) = execute_msg else {
            return false;
        };

        if !is_priority_trader_msg(&trader_msg) {
            return false;
        }
    }

    true
}

fn is_priority_trader_msg(msg: &perps::TraderMsg) -> bool {
    match msg {
        TraderMsg::CancelOrder(_) | TraderMsg::CancelConditionalOrder(_) => true,
        TraderMsg::SubmitOrder(req) => is_post_only(&req.kind),
        TraderMsg::BatchUpdateOrders(batch) => {
            batch.iter().all(is_priority_submit_or_cancel_order_request)
        },
        // Explicit non-priority arms — no `_` wildcard, so adding a new
        // `TraderMsg` variant forces an explicit decision at compile time.
        TraderMsg::Deposit { .. }
        | TraderMsg::Withdraw { .. }
        | TraderMsg::SubmitConditionalOrder { .. } => false,
    }
}

fn is_priority_submit_or_cancel_order_request(req: &SubmitOrCancelOrderRequest) -> bool {
    match req {
        SubmitOrCancelOrderRequest::Cancel(_) => true,
        SubmitOrCancelOrderRequest::Submit(req) => is_post_only(&req.kind),
    }
}

fn is_post_only(kind: &perps::OrderKind) -> bool {
    matches!(kind, perps::OrderKind::Limit {
        time_in_force: perps::TimeInForce::PostOnly,
        ..
    },)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            Dimensionless, Quantity, UsdPrice, UsdValue,
            constants::btc,
            perps::{
                CancelConditionalOrderRequest, CancelOrderRequest, ChildOrder, ExecuteMsg,
                MaintainerMsg, OrderKind, ReferralMsg, SubmitOrCancelOrderRequest,
                SubmitOrderRequest, TimeInForce, TraderMsg, TriggerDirection, VaultMsg,
            },
        },
        grug::{Coins, Json, JsonSerExt, MsgExecute, NonEmpty, Uint64},
        test_case::test_case,
    };

    fn perps() -> Addr {
        Addr::mock(1)
    }

    fn other_contract() -> Addr {
        Addr::mock(2)
    }

    // --- OrderKind constructors -------------------------------------------

    fn limit(tif: TimeInForce) -> OrderKind {
        OrderKind::Limit {
            limit_price: UsdPrice::new_int(100),
            time_in_force: tif,
            client_order_id: None,
        }
    }

    fn post_only_limit() -> OrderKind {
        limit(TimeInForce::PostOnly)
    }

    fn post_only_limit_with_client_id() -> OrderKind {
        OrderKind::Limit {
            limit_price: UsdPrice::new_int(100),
            time_in_force: TimeInForce::PostOnly,
            client_order_id: Some(Uint64::new(42)),
        }
    }

    fn gtc_limit() -> OrderKind {
        limit(TimeInForce::GoodTilCanceled)
    }

    fn ioc_limit() -> OrderKind {
        limit(TimeInForce::ImmediateOrCancel)
    }

    fn market() -> OrderKind {
        OrderKind::Market {
            max_slippage: Dimensionless::new_int(0),
        }
    }

    // --- SubmitOrderRequest ------------------------------------------------

    fn submit(kind: OrderKind) -> SubmitOrderRequest {
        SubmitOrderRequest {
            pair_id: btc::DENOM.clone(),
            size: Quantity::new_int(1),
            kind,
            reduce_only: false,
            tp: None,
            sl: None,
        }
    }

    fn child_order() -> ChildOrder {
        ChildOrder {
            trigger_price: UsdPrice::new_int(120),
            max_slippage: Dimensionless::new_int(0),
            size: None,
        }
    }

    fn submit_with_tp_sl(kind: OrderKind) -> SubmitOrderRequest {
        SubmitOrderRequest {
            pair_id: btc::DENOM.clone(),
            size: Quantity::new_int(1),
            kind,
            reduce_only: false,
            tp: Some(child_order()),
            sl: Some(child_order()),
        }
    }

    // --- TraderMsg constructors --------------------------------------------

    fn cancel_one() -> TraderMsg {
        TraderMsg::CancelOrder(CancelOrderRequest::One(Uint64::new(0)))
    }

    fn cancel_one_by_client_id() -> TraderMsg {
        TraderMsg::CancelOrder(CancelOrderRequest::OneByClientOrderId(Uint64::new(7)))
    }

    fn cancel_all() -> TraderMsg {
        TraderMsg::CancelOrder(CancelOrderRequest::All)
    }

    fn cancel_cond_one() -> TraderMsg {
        TraderMsg::CancelConditionalOrder(CancelConditionalOrderRequest::One {
            pair_id: btc::DENOM.clone(),
            trigger_direction: TriggerDirection::Above,
        })
    }

    fn cancel_cond_all_for_pair() -> TraderMsg {
        TraderMsg::CancelConditionalOrder(CancelConditionalOrderRequest::AllForPair {
            pair_id: btc::DENOM.clone(),
        })
    }

    fn cancel_cond_all() -> TraderMsg {
        TraderMsg::CancelConditionalOrder(CancelConditionalOrderRequest::All)
    }

    fn submit_post_only() -> TraderMsg {
        TraderMsg::SubmitOrder(submit(post_only_limit()))
    }

    fn submit_post_only_with_tp_sl() -> TraderMsg {
        TraderMsg::SubmitOrder(submit_with_tp_sl(post_only_limit()))
    }

    fn submit_post_only_with_client_id() -> TraderMsg {
        TraderMsg::SubmitOrder(submit(post_only_limit_with_client_id()))
    }

    fn submit_market() -> TraderMsg {
        TraderMsg::SubmitOrder(submit(market()))
    }

    fn submit_gtc() -> TraderMsg {
        TraderMsg::SubmitOrder(submit(gtc_limit()))
    }

    fn submit_ioc() -> TraderMsg {
        TraderMsg::SubmitOrder(submit(ioc_limit()))
    }

    fn batch_all_cancel() -> TraderMsg {
        TraderMsg::BatchUpdateOrders(NonEmpty::new_unchecked(vec![
            SubmitOrCancelOrderRequest::Cancel(CancelOrderRequest::All),
            SubmitOrCancelOrderRequest::Cancel(CancelOrderRequest::One(Uint64::new(1))),
        ]))
    }

    fn batch_all_post_only() -> TraderMsg {
        TraderMsg::BatchUpdateOrders(NonEmpty::new_unchecked(vec![
            SubmitOrCancelOrderRequest::Submit(submit(post_only_limit())),
            SubmitOrCancelOrderRequest::Submit(submit(post_only_limit())),
        ]))
    }

    fn batch_mixed_priority() -> TraderMsg {
        TraderMsg::BatchUpdateOrders(NonEmpty::new_unchecked(vec![
            SubmitOrCancelOrderRequest::Cancel(CancelOrderRequest::All),
            SubmitOrCancelOrderRequest::Submit(submit(post_only_limit())),
        ]))
    }

    fn batch_with_market() -> TraderMsg {
        TraderMsg::BatchUpdateOrders(NonEmpty::new_unchecked(vec![
            SubmitOrCancelOrderRequest::Cancel(CancelOrderRequest::All),
            SubmitOrCancelOrderRequest::Submit(submit(market())),
        ]))
    }

    fn batch_with_gtc() -> TraderMsg {
        TraderMsg::BatchUpdateOrders(NonEmpty::new_unchecked(vec![
            SubmitOrCancelOrderRequest::Submit(submit(post_only_limit())),
            SubmitOrCancelOrderRequest::Submit(submit(gtc_limit())),
        ]))
    }

    fn batch_with_ioc() -> TraderMsg {
        TraderMsg::BatchUpdateOrders(NonEmpty::new_unchecked(vec![
            SubmitOrCancelOrderRequest::Submit(submit(ioc_limit())),
        ]))
    }

    fn deposit() -> TraderMsg {
        TraderMsg::Deposit { to: None }
    }

    fn withdraw() -> TraderMsg {
        TraderMsg::Withdraw {
            amount: UsdValue::new_int(1),
        }
    }

    fn submit_conditional() -> TraderMsg {
        TraderMsg::SubmitConditionalOrder {
            pair_id: btc::DENOM.clone(),
            size: None,
            trigger_price: UsdPrice::new_int(100),
            trigger_direction: TriggerDirection::Above,
            max_slippage: Dimensionless::new_int(0),
        }
    }

    // --- Tx builders -------------------------------------------------------

    /// Build a perps trade tx with the given trader messages.
    fn perps_tx(msgs: Vec<TraderMsg>) -> Bytes {
        tx_with_messages(
            msgs.into_iter()
                .map(|tm| Message::execute(perps(), &ExecuteMsg::Trade(tm), Coins::new()).unwrap())
                .collect(),
        )
    }

    fn tx_with_messages(messages: Vec<Message>) -> Bytes {
        let tx = Tx {
            sender: Addr::mock(99),
            gas_limit: 100_000,
            msgs: NonEmpty::new_unchecked(messages),
            data: Json::null(),
            credential: Json::null(),
        };
        tx.to_json_vec().unwrap().into()
    }

    // --- is_priority_trader_msg --------------------------------------------

    #[test_case(cancel_one() => true; "case_cancel_order_one")]
    #[test_case(cancel_one_by_client_id() => true; "case_cancel_order_by_client_id")]
    #[test_case(cancel_all() => true; "case_cancel_order_all")]
    #[test_case(cancel_cond_one() => true; "case_cancel_conditional_one")]
    #[test_case(cancel_cond_all_for_pair() => true; "case_cancel_conditional_all_for_pair")]
    #[test_case(cancel_cond_all() => true; "case_cancel_conditional_all")]
    #[test_case(submit_post_only() => true; "case_submit_post_only")]
    #[test_case(submit_post_only_with_tp_sl() => true; "case_submit_post_only_with_tp_sl")]
    #[test_case(submit_post_only_with_client_id() => true; "case_submit_post_only_with_client_id")]
    #[test_case(batch_all_cancel() => true; "case_batch_all_cancel")]
    #[test_case(batch_all_post_only() => true; "case_batch_all_post_only")]
    #[test_case(batch_mixed_priority() => true; "case_batch_mixed")]
    #[test_case(submit_market() => false; "case_submit_market")]
    #[test_case(submit_gtc() => false; "case_submit_gtc")]
    #[test_case(submit_ioc() => false; "case_submit_ioc")]
    #[test_case(batch_with_market() => false; "case_batch_with_market")]
    #[test_case(batch_with_gtc() => false; "case_batch_with_gtc")]
    #[test_case(batch_with_ioc() => false; "case_batch_with_ioc")]
    #[test_case(deposit() => false; "case_deposit")]
    #[test_case(withdraw() => false; "case_withdraw")]
    #[test_case(submit_conditional() => false; "case_submit_conditional")]
    fn priority_trader_msg(msg: TraderMsg) -> bool {
        is_priority_trader_msg(&msg)
    }

    // --- is_priority_tx ----------------------------------------------------

    #[test_case(perps_tx(vec![cancel_all()])                              => true  ; "single_cancel")]
    #[test_case(perps_tx(vec![submit_post_only()])                        => true  ; "single_post_only")]
    #[test_case(perps_tx(vec![batch_all_cancel()])                        => true  ; "single_batch_all_cancel")]
    #[test_case(perps_tx(vec![cancel_all(), submit_post_only()])          => true  ; "multi_msg_all_priority")]
    #[test_case(perps_tx(vec![cancel_all(), cancel_one(), cancel_cond_all()]) => true ; "multi_cancel_only")]
    #[test_case(perps_tx(vec![submit_market()])                           => false ; "single_market")]
    #[test_case(perps_tx(vec![cancel_all(), submit_market()])             => false ; "multi_msg_one_disqualifies")]
    #[test_case(perps_tx(vec![cancel_all(), deposit()])                   => false ; "multi_msg_deposit_disqualifies")]
    #[test_case(perps_tx(vec![submit_conditional()])                      => false ; "single_conditional_submit")]
    fn priority_tx_perps(tx: Bytes) -> bool {
        is_priority_tx(tx.as_ref(), &perps())
    }

    #[test]
    fn priority_tx_wrong_contract() {
        // A cancel-only tx targeting a different contract is not priority.
        let tx = tx_with_messages(vec![
            Message::execute(
                other_contract(),
                &ExecuteMsg::Trade(cancel_all()),
                Coins::new(),
            )
            .unwrap(),
        ]);
        assert!(!is_priority_tx(tx.as_ref(), &perps()));
    }

    // Top-level `Message` variants other than `Execute` are never priority.
    #[test_case(Message::transfer(perps(), Coins::new()).unwrap() => false ; "case_non_execute_transfer")]
    #[test_case(Message::upload(vec![0u8; 8])                     => false ; "case_non_execute_upload")]
    fn priority_tx_non_execute_message(message: Message) -> bool {
        let tx = tx_with_messages(vec![message]);
        is_priority_tx(tx.as_ref(), &perps())
    }

    // A perps Execute message whose payload is not `Trade(...)` is never
    // priority — covers all sibling variants of `ExecuteMsg`.
    #[test_case(ExecuteMsg::Maintain(MaintainerMsg::Donate {})              => false ; "case_non_trade_maintain")]
    #[test_case(ExecuteMsg::Vault(VaultMsg::Refresh {})                     => false ; "case_non_trade_vault")]
    #[test_case(ExecuteMsg::Referral(ReferralMsg::SetReferral {
        referrer: 1,
        referee: 2,
    })                                                                       => false ; "case_non_trade_referral")]
    fn priority_tx_non_trade_execute_msg(execute_msg: ExecuteMsg) -> bool {
        let tx = tx_with_messages(vec![
            Message::execute(perps(), &execute_msg, Coins::new()).unwrap(),
        ]);
        is_priority_tx(tx.as_ref(), &perps())
    }

    #[test]
    fn priority_tx_malformed_bytes() {
        let raw: &[u8] = b"not a tx";
        assert!(!is_priority_tx(raw, &perps()));
    }

    #[test]
    fn priority_tx_valid_tx_bad_inner_msg() {
        // Valid `Tx`, but `MsgExecute.msg` payload (here, JSON null) isn't a
        // valid perps `ExecuteMsg`.
        let tx = Tx {
            sender: Addr::mock(99),
            gas_limit: 100_000,
            msgs: NonEmpty::new_unchecked(vec![Message::Execute(MsgExecute {
                contract: perps(),
                msg: Json::null(),
                funds: Coins::new(),
            })]),
            data: Json::null(),
            credential: Json::null(),
        };
        let raw: Bytes = tx.to_json_vec().unwrap().into();
        assert!(!is_priority_tx(raw.as_ref(), &perps()));
    }

    // --- promote_priority_txs ----------------------------------------------

    #[test]
    fn promote_empty_input() {
        let txs = promote_priority_txs(vec![], &perps());
        assert!(txs.is_empty());
    }

    #[test]
    fn promote_all_priority_unchanged() {
        let a = perps_tx(vec![cancel_all()]);
        let b = perps_tx(vec![submit_post_only()]);
        let c = perps_tx(vec![batch_all_cancel()]);
        let txs = promote_priority_txs(vec![a.clone(), b.clone(), c.clone()], &perps());
        assert_eq!(txs, vec![a, b, c]);
    }

    #[test]
    fn promote_all_non_priority_unchanged() {
        let a = perps_tx(vec![submit_market()]);
        let b = perps_tx(vec![submit_gtc()]);
        let c = perps_tx(vec![deposit()]);
        let txs = promote_priority_txs(vec![a.clone(), b.clone(), c.clone()], &perps());
        assert_eq!(txs, vec![a, b, c]);
    }

    #[test]
    fn promote_mixed_preserves_relative_order() {
        // Input order: P1, N1, P2, N2, P3
        // Expected output: P1, P2, P3, N1, N2
        let p1 = perps_tx(vec![cancel_all()]);
        let n1 = perps_tx(vec![submit_market()]);
        let p2 = perps_tx(vec![submit_post_only()]);
        let n2 = perps_tx(vec![deposit()]);
        let p3 = perps_tx(vec![cancel_one()]);
        let txs = promote_priority_txs(
            vec![p1.clone(), n1.clone(), p2.clone(), n2.clone(), p3.clone()],
            &perps(),
        );
        assert_eq!(txs, vec![p1, p2, p3, n1, n2]);
    }

    #[test]
    fn promote_single_priority_among_non_priority() {
        let n1 = perps_tx(vec![submit_market()]);
        let p = perps_tx(vec![cancel_all()]);
        let n2 = perps_tx(vec![withdraw()]);
        let txs = promote_priority_txs(vec![n1.clone(), p.clone(), n2.clone()], &perps());
        assert_eq!(txs, vec![p, n1, n2]);
    }
}
