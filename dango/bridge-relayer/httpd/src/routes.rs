use {
    crate::{
        context::Context,
        entity::{self},
    },
    actix_web::{
        Result,
        error::{ErrorBadRequest, ErrorInternalServerError},
        post, web,
    },
    chrono::Utc,
    dango_types::bitcoin::{MultisigWallet, Recipient},
    grug::Addr,
    sea_orm::{ActiveValue::Set, EntityTrait, SqlErr},
    std::str::FromStr,
};

#[post("/deposit-address/{dango_address}")]
async fn deposit_address(path: web::Path<String>, context: web::Data<Context>) -> Result<String> {
    let dango_address = Addr::from_str(&path.into_inner()).map_err(ErrorBadRequest)?;

    // Create the bitcoin deposit address.
    let multisig_wallet = MultisigWallet::new(
        &context.multisig_settings,
        &Recipient::Address(dango_address),
    );
    let bitcoin_deposit_address = multisig_wallet.address(context.network);

    // Store the deposit address in the database.
    let deposit_address = entity::deposit_address::ActiveModel {
        address: Set(bitcoin_deposit_address.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    if let Err(e) = entity::deposit_address::Entity::insert(deposit_address)
        .exec(&context.db)
        .await
    {
        if matches!(e.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
            #[cfg(feature = "tracing")]
            {
                tracing::debug!(%bitcoin_deposit_address, "Deposit address already exists.");
            }
        } else {
            #[cfg(feature = "tracing")]
            {
                tracing::error!(
                    err = e.to_string(),
                    "Failed to store deposit address in database."
                );
            }
            return Err(ErrorInternalServerError(
                "Something went wrong. Please try again later.",
            ));
        }
    };

    Ok(bitcoin_deposit_address.to_string())
}
