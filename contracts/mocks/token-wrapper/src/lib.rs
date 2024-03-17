#[cfg(not(feature = "library"))]
use cw_std::entry_point;
use cw_std::{
    cw_derive, to_json_value, Addr, Coins, Item, Message, MutableCtx, Response, StdResult, SubMessage,
    Uint128,
};

// we namespace all wrapped token denoms with this
// e.g. if a coin has denom `uatom`, we wrap it into `wrapped/uatom`
pub const DENOM_NAMESPACE: &str = "wrapped";

// the bank contract's address
pub const BANK: Item<Addr> = Item::new("bank");

#[cw_derive(serde)]
pub struct InstantiateMsg {
    pub bank: Addr,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    BANK.save(ctx.store, &msg.bank)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn receive(ctx: MutableCtx) -> StdResult<Response> {
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
        submsgs.push(new_refund_msg(ctx.sender, refunds));
    }

    Ok(Response::new().add_submessages(submsgs))
}

fn new_mint_msg(bank: Addr, to: Addr, denom: String, amount: Uint128) -> StdResult<SubMessage> {
    Ok(SubMessage::reply_never(Message::Execute {
        contract: bank,
        msg: to_json_value(&cw_bank::ExecuteMsg::Mint {
            to,
            denom,
            amount,
        })?,
        funds: Coins::new_empty(),
    }))
}

fn new_burn_msg(bank: Addr, from: Addr, denom: String, amount: Uint128) -> StdResult<SubMessage> {
    Ok(SubMessage::reply_never(Message::Execute {
        contract: bank,
        msg: to_json_value(&cw_bank::ExecuteMsg::Burn {
            from,
            denom,
            amount,
        })?,
        funds: Coins::new_empty(),
    }))
}

fn new_refund_msg(to: Addr, coins: Coins) -> SubMessage {
    SubMessage::reply_never(Message::Transfer {
        to,
        coins,
    })
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        cw_std::{hash, Coin, MockStorage, Timestamp, Uint64},
    };

    #[test]
    fn proper_receive() -> anyhow::Result<()> {
        let mut store = MockStorage::new();

        let mock_bank = Addr::mock(1);
        let mock_sender = Addr::mock(2);
        let mock_contract = Addr::mock(3);

        BANK.save(&mut store, &mock_bank)?;

        // TODO: this should be a helper function, something like ReceiveCtx::mock
        let ctx = MutableCtx {
            store:           &mut store,
            chain_id:        "dev-1".into(),
            block_height:    Uint64::new(0),
            block_timestamp: Timestamp::from_seconds(0),
            block_hash:      hash(""),
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
            ),
        ]);

        Ok(())
    }
}
