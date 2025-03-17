use {
    crate::GraphqlCLient,
    grug_types::{Defined, MaybeDefined, Undefined},
    std::ops::Deref,
};

pub struct Client<C, ID = Undefined<String>>
where
    ID: MaybeDefined<String>,
{
    inner: C,
    chain_id: ID,
}

impl Client<GraphqlCLient, Undefined<String>> {
    pub fn new(endpoint: &str) -> Client<GraphqlCLient, Undefined<String>> {
        Self {
            inner: GraphqlCLient::new(endpoint),
            chain_id: Undefined::new(),
        }
    }
}

impl<C> Client<C, Undefined<String>> {
    pub fn enable_broadcasting<CI>(self, chain_id: CI) -> Client<C, Defined<String>>
    where
        CI: Into<String>,
    {
        Client {
            inner: self.inner,
            chain_id: Defined::new(chain_id.into()),
        }
    }
}

impl<C, ID> Deref for Client<C, ID>
where
    ID: MaybeDefined<String>,
{
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use grug_types::QueryClientExt;

    use super::*;

    #[tokio::test]
    async fn graphql_client() {
        let client = Client::new("https://devnet-graphql.dango.exchange");

        let response = client.query_config(None).await.unwrap();
        println!("{:?}", response);
    }
}
