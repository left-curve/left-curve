use crate::entities::pair_price::PairPrice;

const MAX_ITEMS: usize = 100;

#[derive(Debug, Clone)]
pub struct PairPriceResult {
    pub pair_prices: Vec<PairPrice>,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

pub struct PairPriceQueryBuilder {
    base_denom: String,
    quote_denom: String,
    limit: usize,
    later_block_height: Option<u64>,
}

impl PairPriceQueryBuilder {
    pub fn new(base_denom: String, quote_denom: String) -> Self {
        Self {
            base_denom,
            quote_denom,
            limit: MAX_ITEMS,
            later_block_height: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = std::cmp::min(limit, MAX_ITEMS);
        self
    }

    pub fn with_later_block_height(mut self, block_height: u64) -> Self {
        self.later_block_height = Some(block_height);
        self
    }

    pub async fn fetch_all(
        &self,
        clickhouse_client: &clickhouse::Client,
    ) -> Result<PairPriceResult, crate::error::IndexerError> {
        let (query, params, has_previous_page) = self.query_string();

        let mut cursor_query = clickhouse_client.query(&query);
        for param in params {
            cursor_query = cursor_query.bind(param);
        }

        let mut rows: Vec<PairPrice> = cursor_query.fetch_all().await?;

        let has_next_page = rows.len() > self.limit - 1;
        if has_next_page {
            rows.pop();
        }

        Ok(PairPriceResult {
            pair_prices: rows,
            has_next_page,
            has_previous_page,
        })
    }

    pub async fn fetch_one(
        &self,
        clickhouse_client: &clickhouse::Client,
    ) -> Result<Option<PairPrice>, crate::error::IndexerError> {
        let (query, params, _) = self.query_string();

        let mut cursor_query = clickhouse_client.query(&query);
        for param in params {
            cursor_query = cursor_query.bind(param);
        }

        Ok(cursor_query.fetch_optional().await?)
    }

    fn query_string(&self) -> (String, Vec<String>, bool) {
        let has_previous_page = false;

        let mut query = r#"SELECT
                        quote_denom,
                        base_denom,
                        clearing_price,
                        volume_base,
                        volume_quote,
                        created_at,
                        block_height
                       FROM pair_prices
                       WHERE quote_denom = ? AND base_denom = ?"#
            .to_string();

        let mut params: Vec<String> = vec![self.quote_denom.clone(), self.base_denom.clone()];

        if let Some(later_block_height) = self.later_block_height {
            query.push_str(" AND block_height >= ?");
            params.push(later_block_height.to_string());
        }

        query.push_str(" ORDER BY block_height DESC");

        query.push_str(&format!(" LIMIT {}", self.limit + 1));

        (query, params, has_previous_page)
    }
}
