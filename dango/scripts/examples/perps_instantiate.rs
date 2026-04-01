use {
    anyhow::ensure,
    dango_types::{
        Dimensionless, FundingRate, Quantity, UsdPrice, UsdValue,
        constants::{perp_btc, perp_eth, perp_hype, perp_sol},
        perps::{self, PairId, PairParam, Param, RateSchedule},
    },
    grug::{Addr, Coins, ContractWrapper, Duration, HashExt, Message, addr, btree_map, btree_set},
    indexer_client::HttpClient,
    std::collections::BTreeMap,
};

const API_URL: &str = "https://api-testnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("c4a8f7bbadd1457092a8cd182480230c0a848331");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/testnet-owner.json";

fn param() -> Param {
    Param {
        max_unlocks: 5,
        max_open_orders: 20,
        max_conditional_orders: 10,
        maker_fee_rates: RateSchedule {
            base: Dimensionless::new_raw(100), // 0.01%  (1.0 bp)
            tiers: btree_map! {
                UsdValue::new_int(100_000)       => Dimensionless::new_raw(80),   // 0.008% (0.8 bp)
                UsdValue::new_int(1_000_000)     => Dimensionless::new_raw(60),   // 0.006% (0.6 bp)
                UsdValue::new_int(10_000_000)    => Dimensionless::new_raw(40),   // 0.004% (0.4 bp)
                UsdValue::new_int(50_000_000)    => Dimensionless::new_raw(20),   // 0.002% (0.2 bp)
                UsdValue::new_int(200_000_000)   => Dimensionless::ZERO,          // 0%
            },
        },
        taker_fee_rates: RateSchedule {
            base: Dimensionless::new_raw(380), // 0.038% (3.8 bps)
            tiers: btree_map! {
                UsdValue::new_int(100_000)       => Dimensionless::new_raw(320),  // 0.032% (3.2 bps)
                UsdValue::new_int(1_000_000)     => Dimensionless::new_raw(260),  // 0.026% (2.6 bps)
                UsdValue::new_int(10_000_000)    => Dimensionless::new_raw(200),  // 0.020% (2.0 bps)
                UsdValue::new_int(50_000_000)    => Dimensionless::new_raw(160),  // 0.016% (1.6 bps)
                UsdValue::new_int(200_000_000)   => Dimensionless::new_raw(140),  // 0.014% (1.4 bps)
            },
        },
        protocol_fee_rate: Dimensionless::new_percent(25), // 25% to treasury, 75% to vault
        liquidation_fee_rate: Dimensionless::new_permille(5), // 0.5% of notional
        funding_period: Duration::from_hours(1),
        vault_total_weight: Dimensionless::new_int(10), // 4 + 3 + 2 + 1
        vault_cooldown_period: Duration::from_days(3),
        referral_active: true,
        min_referrer_volume: UsdValue::new_int(10_000),
        referrer_commission_rates: RateSchedule {
            base: Dimensionless::new_percent(10), // 10% of referee fees
            tiers: btree_map! {
                UsdValue::new_int(10_000_000)  => Dimensionless::new_percent(15), // 15% at $10M+
                UsdValue::new_int(30_000_000)  => Dimensionless::new_percent(20), // 20% at $30M+
                UsdValue::new_int(50_000_000)  => Dimensionless::new_percent(25), // 25% at $50M+
                UsdValue::new_int(100_000_000) => Dimensionless::new_percent(30), // 30% at $100M+
            },
        },
    }
}

fn pair_params() -> BTreeMap<PairId, PairParam> {
    btree_map! {
            // ---------------------------------------------------------------
            // BTC-USD  (max leverage 20x)
            // ---------------------------------------------------------------
            perp_btc::DENOM.clone() => PairParam {
                initial_margin_ratio:      Dimensionless::new_percent(5),     // 5%  -> 20x
                maintenance_margin_ratio:  Dimensionless::new_raw(25_000),    // 2.5%
                max_abs_oi:                Quantity::new_int(500),            // 500 BTC
                max_abs_funding_rate:      FundingRate::new_raw(16_670),      // ~1.667%/day (IMR/3)
                vault_half_spread:         Dimensionless::new_raw(500),       // 0.05% (5 bps)
                vault_max_quote_size:      Quantity::new_int(200),            // 200 BTC
                // -- non-risk (shared) --
                tick_size:                 UsdPrice::new_int(1),              // $1
                min_order_size:            UsdValue::new_int(50),             // $50
                impact_size:               UsdValue::new_int(50_000),         // $50k
                vault_liquidity_weight:    Dimensionless::new_int(4),
                bucket_sizes:              btree_set! {
                    UsdPrice::new_int(1),
                    UsdPrice::new_int(10),
                    UsdPrice::new_int(100),
                },
            },

            // ---------------------------------------------------------------
            // ETH-USD  (max leverage 20x)
            // ---------------------------------------------------------------
            perp_eth::DENOM.clone() => PairParam {
                initial_margin_ratio:      Dimensionless::new_percent(5),     // 5%  -> 20x
                maintenance_margin_ratio:  Dimensionless::new_raw(25_000),    // 2.5%
                max_abs_oi:                Quantity::new_int(5_000),          // 5,000 ETH
                max_abs_funding_rate:      FundingRate::new_raw(16_670),      // ~1.667%/day
                vault_half_spread:         Dimensionless::new_raw(600),       // 0.06% (6 bps)
                vault_max_quote_size:      Quantity::new_int(2_000),          // 2,000 ETH
                // -- non-risk (shared) --
                tick_size:                 UsdPrice::new_raw(100_000),        // $0.10
                min_order_size:            UsdValue::new_int(25),             // $25
                impact_size:               UsdValue::new_int(25_000),         // $25k
                vault_liquidity_weight:    Dimensionless::new_int(3),
                bucket_sizes:              btree_set! {
                    UsdPrice::new_raw(100_000),   // $0.10
                    UsdPrice::new_int(1),         // $1
                    UsdPrice::new_int(10),        // $10
                },
            },

            // ---------------------------------------------------------------
            // SOL-USD  (max leverage 10x)
            // ---------------------------------------------------------------
            perp_sol::DENOM.clone() => PairParam {
                initial_margin_ratio:      Dimensionless::new_percent(10),    // 10% -> 10x
                maintenance_margin_ratio:  Dimensionless::new_percent(5),     // 5%
                max_abs_oi:                Quantity::new_int(100_000),        // 100k SOL
                max_abs_funding_rate:      FundingRate::new_raw(33_330),      // ~3.333%/day
                vault_half_spread:         Dimensionless::new_permille(1),    // 0.10% (10 bps)
                vault_max_quote_size:      Quantity::new_int(40_000),         // 40k SOL
                // -- non-risk (shared) --
                tick_size:                 UsdPrice::new_raw(10_000),         // $0.01
                min_order_size:            UsdValue::new_int(10),             // $10
                impact_size:               UsdValue::new_int(10_000),         // $10k
                vault_liquidity_weight:    Dimensionless::new_int(2),
                bucket_sizes:              btree_set! {
                    UsdPrice::new_raw(10_000),    // $0.01
                    UsdPrice::new_raw(100_000),   // $0.10
                    UsdPrice::new_int(1),         // $1
                },
            },

            // ---------------------------------------------------------------
            // HYPE-USD  (max leverage 5x)
            // ---------------------------------------------------------------
            perp_hype::DENOM.clone() => PairParam {
                initial_margin_ratio:      Dimensionless::new_percent(20),    // 20% -> 5x
                maintenance_margin_ratio:  Dimensionless::new_percent(10),    // 10%
                max_abs_oi:                Quantity::new_int(200_000),        // 200k HYPE
                max_abs_funding_rate:      FundingRate::new_raw(66_670),      // ~6.667%/day (IMR/3)
                vault_half_spread:         Dimensionless::new_raw(1_500),     // 0.15% (15 bps)
                vault_max_quote_size:      Quantity::new_int(80_000),         // 80k HYPE
                // -- non-risk (shared) --
                tick_size:                 UsdPrice::new_raw(10_000),         // $0.01
                min_order_size:            UsdValue::new_int(10),             // $10
                impact_size:               UsdValue::new_int(5_000),          // $5k
                vault_liquidity_weight:    Dimensionless::new_int(1),
                bucket_sizes:              btree_set! {
                    UsdPrice::new_raw(10_000),    // $0.01
                    UsdPrice::new_raw(100_000),   // $0.10
                    UsdPrice::new_int(1),         // $1
                },
            }
    }
}

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        let code = ContractWrapper::from_index(13).to_bytes();
        let code_hash = code.hash256();

        ensure!(
            code_hash.to_string()
                == "B0BD73E6922C0D2496DBCB99EAC08EB5870090E59F6E06B1EB477D540002A5CD",
            "code hash is not the same as what we uploaded in the previous step"
        );

        Ok(Message::instantiate(
            code_hash,
            &perps::InstantiateMsg {
                param: param(),
                pair_params: pair_params(),
            },
            "dango/perps",
            Some("dango/perps"),
            None,
            Coins::new(),
        )?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
