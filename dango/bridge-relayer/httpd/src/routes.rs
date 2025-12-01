use {
    crate::{
        context::Context,
        entity::{self},
        middlewares,
    },
    actix_web::{
        HttpResponseBuilder, Responder, Result,
        error::{ErrorBadRequest, InternalError},
        get,
        http::StatusCode,
        post, web,
    },
    chrono::Utc,
    dango_types::bitcoin::{MultisigWallet, Recipient},
    grug::Addr,
    metrics::counter,
    sea_orm::{ActiveValue::Set, EntityTrait, PaginatorTrait, QueryOrder, SqlErr},
    serde::{Deserialize, Serialize},
    std::str::FromStr,
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
            return Err(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong. Please try again later.",
            ));
        }
    } else {
        counter!(middlewares::metrics::LABEL_DEPOSIT_ADDRESS_TOTAL).increment(1);
    }

    Ok(bitcoin_deposit_address.to_string())
}

#[derive(Deserialize, Serialize)]
pub struct DepositAddressesRequest {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Deserialize, Serialize)]
pub struct DepositAddressesResponse {
    pub addresses: Vec<String>,
    pub next_page: Option<u64>,
}

pub const DEFAULT_PAGE_LIMIT: u64 = 1000;

#[get("/deposit-addresses")]
async fn deposit_addresses(
    info: web::Query<DepositAddressesRequest>,
    context: web::Data<Context>,
) -> Result<impl Responder> {
    let limit = info.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    let page = info.page.unwrap_or(0);

    let paginator = entity::deposit_address::Entity::find()
        .order_by_asc(entity::deposit_address::Column::Id)
        .paginate(&context.db, limit);

    let num_pages = paginator.num_pages().await.map_err(|e| {
        #[cfg(feature = "tracing")]
        {
            tracing::error!(err = e.to_string(), "Failed to fetch deposit addresses.");
        }
        json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong. Please try again later.",
        )
    })?;

    if page >= num_pages {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            &format!("Invalid page number. Page number must be less than {num_pages}."),
        ));
    }

    #[cfg(feature = "tracing")]
    {
        tracing::debug!(page, limit, num_pages, "Fetching deposit addresses.");
    }

    let addresses: Vec<String> = paginator
        .fetch_page(page)
        .await
        .map_err(|e| {
            #[cfg(feature = "tracing")]
            {
                tracing::error!(err = e.to_string(), "Failed to fetch deposit addresses.");
            }
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong. Please try again later.",
            )
        })?
        .into_iter()
        .map(|model| model.address)
        .collect();

    let next_page = if page < num_pages - 1 {
        Some(page + 1)
    } else {
        None
    };

    let res = DepositAddressesResponse {
        addresses,
        next_page,
    };
    Ok(web::Json(res))
}
