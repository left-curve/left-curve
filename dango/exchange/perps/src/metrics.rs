// ----------------------------- Order flow ------------------------------------

pub const LABEL_ORDERS_SUBMITTED: &str = "dango.contract.perps.orders_submitted_count";

pub const LABEL_ORDERS_FILLED: &str = "dango.contract.perps.orders_filled_count";

pub const LABEL_TRADES: &str = "dango.contract.perps.trades_count";

pub const LABEL_VOLUME_PER_TRADE: &str = "dango.contract.perps.volume_per_trade";

pub const LABEL_FEES_COLLECTED: &str = "dango.contract.perps.fees_collected";

// ----------------------------- Deposit / Withdraw ----------------------------

pub const LABEL_DEPOSIT_AMOUNT: &str = "dango.contract.perps.deposit_amount";

pub const LABEL_WITHDRAWAL_AMOUNT: &str = "dango.contract.perps.withdrawal_amount";

pub const LABEL_VAULT_DEPOSIT_AMOUNT: &str = "dango.contract.perps.vault_deposit_amount";

pub const LABEL_VAULT_WITHDRAWAL_AMOUNT: &str = "dango.contract.perps.vault_withdrawal_amount";

// ----------------------------- Liquidation -----------------------------------

pub const LABEL_LIQUIDATIONS: &str = "dango.contract.perps.liquidations_count";

pub const LABEL_ADL_EVENTS: &str = "dango.contract.perps.adl_events_count";

pub const LABEL_BAD_DEBT: &str = "dango.contract.perps.bad_debt";

// ----------------------------- Open interest ---------------------------------

pub const LABEL_OPEN_INTEREST_LONG: &str = "dango.contract.perps.open_interest_long";

pub const LABEL_OPEN_INTEREST_SHORT: &str = "dango.contract.perps.open_interest_short";

// ----------------------------- Vault -----------------------------------------

/// Vault equity in USD, updated once per block in cron.
pub const LABEL_VAULT_EQUITY: &str = "dango.contract.perps.vault_equity";

pub const LABEL_VAULT_MARGIN: &str = "dango.contract.perps.vault_margin";

pub const LABEL_VAULT_POSITION: &str = "dango.contract.perps.vault_position";

pub const LABEL_VAULT_SHARE_SUPPLY: &str = "dango.contract.perps.vault_share_supply";

pub const LABEL_INSURANCE_FUND: &str = "dango.contract.perps.insurance_fund";

pub const LABEL_TREASURY: &str = "dango.contract.perps.treasury";

// ----------------------------- Funding ---------------------------------------

pub const LABEL_FUNDING_RATE: &str = "dango.contract.perps.funding_rate";

pub const LABEL_FUNDING_PER_UNIT: &str = "dango.contract.perps.funding_per_unit";

// ----------------------------- Durations -------------------------------------

pub const LABEL_DURATION_SUBMIT_ORDER: &str = "dango.contract.perps.submit_order.duration";

pub const LABEL_DURATION_LIQUIDATE: &str = "dango.contract.perps.liquidate.duration";

pub const LABEL_DURATION_CRON: &str = "dango.contract.perps.cron.duration";

pub const LABEL_DURATION_VAULT_REFRESH: &str = "dango.contract.perps.vault_refresh.duration";

pub fn init_metrics() {
    use {
        metrics::{describe_counter, describe_gauge, describe_histogram},
        std::sync::Once,
    };

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        // Order flow
        describe_counter!(LABEL_ORDERS_SUBMITTED, "Number of orders submitted");
        describe_counter!(LABEL_ORDERS_FILLED, "Number of orders fully filled");
        describe_counter!(LABEL_TRADES, "Number of trades executed (individual fills)");
        describe_histogram!(LABEL_VOLUME_PER_TRADE, "Notional volume per trade in USD");
        describe_histogram!(
            LABEL_FEES_COLLECTED,
            "Trading fees collected per trade in USD"
        );

        // Deposit / Withdraw
        describe_histogram!(LABEL_DEPOSIT_AMOUNT, "Deposit amount in USD");
        describe_histogram!(LABEL_WITHDRAWAL_AMOUNT, "Withdrawal amount in USD");
        describe_histogram!(
            LABEL_VAULT_DEPOSIT_AMOUNT,
            "Vault deposit (add liquidity) amount in USD"
        );
        describe_histogram!(
            LABEL_VAULT_WITHDRAWAL_AMOUNT,
            "Vault withdrawal (remove liquidity) amount in USD"
        );

        // Liquidation
        describe_counter!(LABEL_LIQUIDATIONS, "Number of liquidations triggered");
        describe_counter!(LABEL_ADL_EVENTS, "Number of ADL (auto-deleverage) events");
        describe_histogram!(LABEL_BAD_DEBT, "Bad debt absorbed by insurance fund in USD");

        // Open interest
        describe_gauge!(LABEL_OPEN_INTEREST_LONG, "Long open interest per pair");
        describe_gauge!(LABEL_OPEN_INTEREST_SHORT, "Short open interest per pair");

        // Vault / global state
        describe_gauge!(LABEL_VAULT_EQUITY, "Vault equity in USD");
        describe_gauge!(LABEL_VAULT_MARGIN, "Vault deposited margin in USD");
        describe_gauge!(
            LABEL_VAULT_POSITION,
            "Vault position size per pair (positive=long, negative=short)"
        );
        describe_gauge!(LABEL_VAULT_SHARE_SUPPLY, "Total supply of vault shares");
        describe_gauge!(LABEL_INSURANCE_FUND, "Insurance fund balance in USD");
        describe_gauge!(LABEL_TREASURY, "Protocol treasury balance in USD");

        // Funding
        describe_gauge!(LABEL_FUNDING_RATE, "Current funding rate per pair");
        describe_gauge!(
            LABEL_FUNDING_PER_UNIT,
            "Cumulative funding per unit of asset per pair"
        );

        // Durations
        describe_histogram!(
            LABEL_DURATION_SUBMIT_ORDER,
            "Time spent processing a submit-order"
        );
        describe_histogram!(
            LABEL_DURATION_LIQUIDATE,
            "Time spent processing a liquidation"
        );
        describe_histogram!(LABEL_DURATION_CRON, "Time spent on cron execution");
        describe_histogram!(
            LABEL_DURATION_VAULT_REFRESH,
            "Time spent refreshing vault orders"
        );
    });
}
