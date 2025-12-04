use {
    crate::{
        context::Context,
        entity::{self},
        middlewares,
    },
    actix_web::{
        HttpResponse, HttpResponseBuilder, Responder, Result,
        error::{ErrorBadRequest, InternalError},
        get,
        http::StatusCode,
        post, web,
    },
    async_stream::stream,
    chrono::Utc,
    dango_types::bitcoin::{MultisigWallet, Recipient},
    grug::Addr,
    metrics::counter,
    sea_orm::{
        ActiveValue::{NotSet, Set},
        ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QueryTrait,
        sea_query::OnConflict,
    },
    serde::{Deserialize, Serialize},
    std::{
        str::FromStr,
        sync::{Arc, Mutex},
    },
    tokio_stream::StreamExt,
};

#[derive(Deserialize, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Create a JSON error response.
fn json_error(status_code: StatusCode, error: &str) -> actix_web::Error {
    InternalError::from_response(
        error.to_string(),
        HttpResponseBuilder::new(status_code)
            .content_type("application/json")
            .body(
                serde_json::to_string(&ErrorResponse {
                    error: error.to_string(),
                })
                .unwrap(),
            ),
    )
    .into()
}

#[post("/deposit-address/{dango_address}")]
async fn deposit_address(path: web::Path<String>, context: web::Data<Context>) -> Result<String> {
    let dango_address = Addr::from_str(&path.into_inner()).map_err(ErrorBadRequest)?;

    #[cfg(feature = "tracing")]
    {
        tracing::debug!(%dango_address, "Requesting new deposit address.");
    }

    // Create the bitcoin deposit address.
    let multisig_wallet = MultisigWallet::new(
        &context.multisig_settings,
        &Recipient::Address(dango_address),
    );
    let bitcoin_deposit_address = multisig_wallet.address(context.network);

    let created_at = Utc::now().timestamp_millis();

    // Store the deposit address in the database.
    let deposit_address = entity::deposit_address::ActiveModel {
        address: Set(bitcoin_deposit_address.to_string()),
        created_at: Set(created_at),
        id: NotSet,
    };
    if let Err(e) = entity::deposit_address::Entity::insert(deposit_address)
        .on_conflict(
            OnConflict::column(entity::deposit_address::Column::Address)
                .update_column(entity::deposit_address::Column::CreatedAt)
                .value(entity::deposit_address::Column::CreatedAt, created_at)
                .to_owned(),
        )
        .exec(&context.db)
        .await
    {
        #[cfg(feature = "tracing")]
        {
            tracing::error!(
                err = e.to_string(),
                "Failed to store or update deposit address in database."
            );
        }
        return Err(json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong. Please try again later.",
        ));
    } else {
        #[cfg(feature = "tracing")]
        {
            tracing::info!(%bitcoin_deposit_address, %created_at, "Deposit address stored or updated in database.");
        }

        #[cfg(feature = "metrics")]
        counter!(middlewares::metrics::LABEL_DEPOSIT_ADDRESS_TOTAL).increment(1);
    }

    Ok(bitcoin_deposit_address.to_string())
}

#[derive(Deserialize, Serialize)]
pub struct DepositAddressesRequest {
    /// The unix timestamp in milliseconds of the time at which the deposit address after which to fetch.
    pub after_created_at: Option<u64>,
}

#[get("/deposit-addresses")]
async fn deposit_addresses(
    info: web::Query<DepositAddressesRequest>,
    context: web::Data<Context>,
) -> Result<impl Responder> {
    let after_created_at = info.after_created_at;
    let db = context.db.clone();

    #[cfg(feature = "tracing")]
    {
        tracing::info!(after_created_at = ?after_created_at, "Fetching deposit addresses.");
    }

    let is_first = Arc::new(Mutex::new(true));

    let response_stream = stream! {
        // First, yield the opening bracket
        yield Ok::<_, actix_web::Error>(web::Bytes::from("["));

        // Create the database stream
        let mut db_stream = match entity::deposit_address::Entity::find().apply_if(after_created_at, |query, v| {
            query.filter(entity::deposit_address::Column::CreatedAt.gt(v))
        })
            .order_by_asc(entity::deposit_address::Column::CreatedAt)
            .stream(&db)
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(err = e.to_string(), "Failed to fetch deposit addresses.");
                }
                yield Err(json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong. Please try again later.",
                ));
                return;
            }
        };

        // Stream the results
        while let Some(result) = db_stream.next().await {
            match result {
                Ok(model) => {
                    let mut first = is_first.lock().unwrap();
                    let prefix = if *first {
                        *first = false;
                        ""
                    } else {
                        ","
                    };
                    let json = serde_json::to_string(&model.address).unwrap();
                    yield Ok(web::Bytes::from(format!("{}{}", prefix, json)));
                }
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    {
                        tracing::error!(err = e.to_string(), "Error streaming deposit addresses.");
                    }
                    yield Err(json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Something went wrong. Please try again later.",
                    ));
                    return;
                }
            }
        }

        // Finally, yield the closing bracket
        yield Ok(web::Bytes::from("]"));
    };

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .streaming(response_stream))
}
