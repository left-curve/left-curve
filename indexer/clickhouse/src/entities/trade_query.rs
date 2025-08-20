use {crate::entities::trade::Trade, dango_types::dex::PairId, grug::Addr};

const MAX_ITEMS: usize = 100;

#[derive(Debug, Clone)]
pub struct TradeResult {
    pub trades: Vec<Trade>,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

pub struct TradeQueryBuilder {
    pair: Option<PairId>,
    limit: usize,
    later_block_height: Option<u64>,
    addr: Option<Addr>,
}

impl Default for TradeQueryBuilder {
    fn default() -> Self {
        Self {
            limit: MAX_ITEMS,
            pair: Default::default(),
            later_block_height: Default::default(),
            addr: Default::default(),
        }
    }
}

impl TradeQueryBuilder {
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = std::cmp::min(limit, MAX_ITEMS);
        self
    }

    pub fn with_addr(mut self, addr: Addr) -> Self {
        self.addr = Some(addr);
        self
    }

    pub fn with_pair(mut self, pair: PairId) -> Self {
        self.pair = Some(pair);
        self
    }

    pub fn with_later_block_height(mut self, block_height: u64) -> Self {
        self.later_block_height = Some(block_height);
        self
    }

    pub async fn fetch_all(
        &self,
        clickhouse_client: &clickhouse::Client,
    ) -> Result<TradeResult, crate::error::IndexerError> {
        let (query, params, has_previous_page) = self.query_string();

        let mut cursor_query = clickhouse_client.query(&query);
        for param in params {
            cursor_query = cursor_query.bind(param);
        }

        let mut rows: Vec<Trade> = cursor_query.fetch_all().await?;

        let has_next_page = rows.len() > self.limit - 1;
        if has_next_page {
            rows.pop();
        }

        Ok(TradeResult {
            trades: rows,
            has_next_page,
            has_previous_page,
        })
    }

    pub async fn fetch_one(
        &self,
        clickhouse_client: &clickhouse::Client,
    ) -> Result<Option<Trade>, crate::error::IndexerError> {
        let (query, params, _) = self.query_string();

        let mut cursor_query = clickhouse_client.query(&query);
        for param in params {
            cursor_query = cursor_query.bind(param);
        }

        Ok(cursor_query.fetch_optional().await?)
    }

    fn query_string(&self) -> (String, Vec<String>, bool) {
        let has_previous_page = false;

        let mut query = r#"
          SELECT
            quote_denom,
            base_denom,
            addr,
            direction,
            filled_base,
            filled_quote,
            refund_base,
            refund_quote,
            fee_base,
            fee_quote,
            clearing_price,
            created_at,
            block_height
          FROM trades
          WHERE 1=1
        "#
        .to_string();

        let mut params: Vec<String> = vec![];

        if let Some(pair) = &self.pair {
            query.push_str(" AND base_denom = ? AND quote_denom = ?");
            params.push(pair.base_denom.to_string());
            params.push(pair.quote_denom.to_string());
        }

        if let Some(later_block_height) = self.later_block_height {
            query.push_str(" AND block_height >= ?");
            params.push(later_block_height.to_string());
        }

        if let Some(addr) = &self.addr {
            query.push_str(" AND addr = ?");
            params.push(addr.to_string());
        }

        query.push_str(" ORDER BY block_height DESC");
        query.push_str(&format!(" LIMIT {}", self.limit + 1));

        (query, params, has_previous_page)
    }
}
