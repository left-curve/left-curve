use alloy::{primitives::Address, providers::Provider};

pub async fn is_contract<P: Provider>(provider: &P, address: Address) -> anyhow::Result<bool> {
    let code = provider.get_code_at(address).await?;
    Ok(!code.is_empty())
}
