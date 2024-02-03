#[cfg(not(feature = "library"))]
use cw_std::entry_point;
use cw_std::{
    cw_serde, to_json, Addr, Coins, GenericResult, InstantiateCtx, Item, Message, ReceiveCtx,
    ReplyCtx, Response, StdResult, SubMessage, Uint128,
};

// we namespace all wrapped token denoms with this
// e.g. if a coin has denom `uatom`, we wrap it into `wrapped/uatom`
pub const DENOM_NAMESPACE: &str = "wrapped";

// the bank contract's address
pub const BANK: Item<Addr> = Item::new("bank");

#[cw_serde]
pub struct InstantiateMsg {
    pub bank: Addr,
}

#[cw_serde]
pub enum ReplyMsg {
    AfterMint,
    AfterBurn,
    AfterRefund,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(ctx: InstantiateCtx, msg: InstantiateMsg) -> StdResult<Response> {
    BANK.save(ctx.store, &msg.bank)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn receive(ctx: ReceiveCtx) -> StdResult<Response> {
    let bank = BANK.load(ctx.store)?;

    let mut submsgs = vec![];
    let mut refunds = Coins::new_empty();

    for coin in ctx.funds {
        match coin.denom.split_once('/') {
            // we're sent a wrapped token. unwrap it, and refund the sender the
            // underlying coin
            Some((DENOM_NAMESPACE, suffix)) => {
                submsgs.push(new_burn_msg(
                    bank.clone(),
                    ctx.contract.clone(),
                    coin.denom.clone(),
                    coin.amount,
                )?);
                refunds.increase_amount(suffix, coin.amount)?;
            },
            // not a wrapped token. wrap it
            _ => {
                submsgs.push(new_mint_msg(
                    bank.clone(),
                    ctx.sender.clone(),
                    format!("{DENOM_NAMESPACE}/{}", coin.denom),
                    coin.amount,
                )?);
            },
        }
    }

    if !refunds.is_empty() {
        submsgs.push(new_refund_msg(ctx.sender, refunds)?);
    }

    Ok(Response::new().add_submessages(submsgs))
}

// for this demo, there is no action to be taken in the reply. we just log some
// event attributes.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(ctx: ReplyCtx, msg: ReplyMsg) -> StdResult<Response> {
    let method = match msg {
        ReplyMsg::AfterMint => "after_mint",
        ReplyMsg::AfterBurn => "after_burn",
        ReplyMsg::AfterRefund => "after_refund",
    };

    let submsg_result = match ctx.submsg_result {
        GenericResult::Ok(_) => "ok",
        GenericResult::Err(_) => "err",
    };

    Ok(Response::new()
        .add_attribute("method", method)
        .add_attribute("submsg_result", submsg_result))
}

fn new_mint_msg(bank: Addr, to: Addr, denom: String, amount: Uint128) -> StdResult<SubMessage> {
    SubMessage::reply_always(
        Message::Execute {
            contract: bank,
            msg: to_json(&cw_bank::ExecuteMsg::Mint {
                to,
                denom,
                amount,
            })?,
            funds: Coins::new_empty(),
        },
        &ReplyMsg::AfterMint,
    )
}

fn new_burn_msg(bank: Addr, from: Addr, denom: String, amount: Uint128) -> StdResult<SubMessage> {
    SubMessage::reply_always(
        Message::Execute {
            contract: bank,
            msg: to_json(&cw_bank::ExecuteMsg::Burn {
                from,
                denom,
                amount,
            })?,
            funds: Coins::new_empty(),
        },
        &ReplyMsg::AfterBurn,
    )
}

fn new_refund_msg(to: Addr, coins: Coins) -> StdResult<SubMessage> {
    SubMessage::reply_always(
        Message::Transfer {
            to,
            coins,
        },
        &ReplyMsg::AfterRefund,
    )
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        cw_std::{Coin, MockStorage, Timestamp, Uint64},
    };

    #[test]
    fn proper_receive() -> anyhow::Result<()> {
        let mut store = MockStorage::new();

        let mock_bank = Addr::mock(1);
        let mock_sender = Addr::mock(2);
        let mock_contract = Addr::mock(3);

        BANK.save(&mut store, &mock_bank)?;

        // TODO: this should be a helper function, something like ReceiveCtx::mock
        let ctx = ReceiveCtx {
            store:           &mut store,
            chain_id:        "dev-1".into(),
            block_height:    Uint64::new(0),
            block_timestamp: Timestamp::from_seconds(0),
            contract:        mock_contract.clone(),
            sender:          mock_sender.clone(),
            // note that coins must be sorted by denom
            funds: Coins::from_vec_unchecked(vec![
                // no prefix
                Coin::new("uosmo", 111),
                // have a prefix and the it's `wrapped`
                Coin::new("wrapped/uatom", 222),
                // one more wrapped token
                Coin::new("wrapped/umars", 333),
                // has a prefix but it's not `wrapped`
                Coin::new("zzz/haha", 444),
            ]),
        };

        let res = receive(ctx)?;
        assert_eq!(res.submsgs, vec![
            new_mint_msg(
                mock_bank.clone(),
                mock_sender.clone(),
                "wrapped/uosmo".into(),
                Uint128::new(111),
            )?,
            new_burn_msg(
                mock_bank.clone(),
                mock_contract.clone(),
                "wrapped/uatom".into(),
                Uint128::new(222),
            )?,
            new_burn_msg(
                mock_bank.clone(),
                mock_contract.clone(),
                "wrapped/umars".into(),
                Uint128::new(333),
            )?,
            new_mint_msg(
                mock_bank.clone(),
                mock_sender.clone(),
                "wrapped/zzz/haha".into(),
                Uint128::new(444),
            )?,
            new_refund_msg(
                mock_sender,
                Coins::from_vec_unchecked(vec![
                    Coin::new("uatom", 222),
                    Coin::new("umars", 333),
                ]),
            )?,
        ]);

        Ok(())
    }
}
