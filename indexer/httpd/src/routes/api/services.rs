use {
    super::{
        blocks::{block_info_by_height, block_result, block_result_by_height, latest_block_info},
        tendermint::search_tx,
    },
    actix_web::{Scope, web},
};

pub fn api_services() -> Scope {
    web::scope("/api")
        .service(block_services())
        .service(tendermint_services())
}

fn block_services() -> Scope {
    web::scope("/block")
        .service(block_info_by_height)
        .service(latest_block_info)
        .service(block_result_by_height)
        .service(block_result)
}

fn tendermint_services() -> Scope {
    web::scope("/tendermint").service(search_tx)
}
