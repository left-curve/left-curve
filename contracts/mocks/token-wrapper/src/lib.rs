use cw_std::{
    cw_serde, entry_point, to_json, Addr, Coins, InstantiateCtx, Item, Message, ReceiveCtx,
    Response, Uint128,
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(ctx: InstantiateCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    BANK.save(ctx.store, &msg.bank)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn receive(ctx: ReceiveCtx) -> anyhow::Result<Response> {
    let bank = BANK.load(ctx.store)?;

    let mut msgs = vec![];
    let mut refunds = Coins::new_empty();

    for coin in ctx.funds {
        match coin.denom.split_once('/') {
            // we're sent a wrapped token. unwrap it, and refund the sender the
            // underlying coin
            Some((DENOM_NAMESPACE, suffix)) => {
                msgs.push(new_burn_msg(
                    bank.clone(),
                    ctx.contract.clone(),
                    coin.denom.clone(),
                    coin.amount,
                )?);
                refunds.increase_amount(suffix, coin.amount)?;
            },
            // not a wrapped token. wrap it
            _ => {
                msgs.push(new_mint_msg(
                    bank.clone(),
                    ctx.sender.clone(),
                    format!("{DENOM_NAMESPACE}/{}", coin.denom),
                    coin.amount,
                )?);
            },
        }
    }

    if !refunds.is_empty() {
        msgs.push(Message::Transfer {
            to:    ctx.sender,
            coins: refunds,
        });
    }

    Ok(Response::new().add_messages(msgs))
}

fn new_mint_msg(bank: Addr, to: Addr, denom: String, amount: Uint128) -> anyhow::Result<Message> {
    Ok(Message::Execute {
        contract: bank,
        msg: to_json(&cw_bank::ExecuteMsg::Mint {
            to,
            denom,
            amount,
        })?,
        funds: Coins::new_empty(),
    })
}

fn new_burn_msg(bank: Addr, from: Addr, denom: String, amount: Uint128) -> anyhow::Result<Message> {
    Ok(Message::Execute {
        contract: bank,
        msg: to_json(&cw_bank::ExecuteMsg::Burn {
            from,
            denom,
            amount,
        })?,
        funds: Coins::new_empty(),
    })
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        cw_std::{BlockInfo, Coin, MockStorage},
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
            store: &mut store,
            chain_id: "dev-1".into(),
            block: BlockInfo {
                height:    0,
                timestamp: 0,
            },
            contract: mock_contract.clone(),
            sender: mock_sender.clone(),
            // note that coins are sorted
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
        assert_eq!(res.msgs, vec![
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
            Message::Transfer {
                to: mock_sender,
                coins: Coins::from_vec_unchecked(vec![
                    Coin::new("uatom", 222),
                    Coin::new("umars", 333),
                ]),
            },
        ]);

        Ok(())
    }
}
