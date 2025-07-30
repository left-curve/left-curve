use {
    dango_types::{config::AppConfig, dex, oracle},
    grug::{Dec, QueryClientExt, Udec128_6},
    indexer_client::HttpClient,
    num_format::{Locale, ToFormattedString},
    std::fmt::Display,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new("https://testnet.dango.exchange");
    let cfg: AppConfig = client.query_app_config(None).await?;

    let res = client
        .query_wasm_smart(
            cfg.addresses.dex,
            dex::QueryReservesRequest {
                start_after: None,
                limit: None,
            },
            None,
        )
        .await?;

    let prices = client
        .query_wasm_smart(
            cfg.addresses.oracle,
            oracle::QueryPricesRequest {
                start_after: None,
                limit: None,
            },
            None,
        )
        .await?;

    for reserve in res {
        println!(
            "pool {}/{}",
            reserve.pair.base_denom, reserve.pair.quote_denom
        );

        let status = reserve
            .reserve
            .into_iter()
            .map(|i| {
                let pr = prices.get(&i.denom).unwrap();
                let value: grug::Dec<u128, 6> = pr.value_of_unit_amount(i.amount).unwrap();
                format!(
                    "{}: {:_<3} (${:_<3})",
                    i.denom,
                    Udec128_6::checked_from_ratio(i.amount.0, 10_u128.pow(pr.precision() as u32))
                        .unwrap()
                        .to_nice_string(),
                    value.to_nice_string()
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");

        println!("{status}",);
        println!();
    }

    Ok(())
}

pub trait NiceString {
    fn to_nice_string(&self) -> String;
}

impl<U, const S: u32> NiceString for Dec<U, S>
where
    Dec<U, S>: Display,
{
    fn to_nice_string(&self) -> String {
        let str_self = self.to_string();
        let mut iter = str_self.split(".");

        let buff = iter
            .next()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            .to_formatted_string(&Locale::en);

        if let Some(decimal_part) = iter.next() {
            format!("{buff}.{decimal_part}")
        } else {
            buff
        }
    }
}
