use {
    super::blocks::{
        block_info_by_height, block_result, block_result_by_height, latest_block_info,
    },
    actix_web::{Scope, web},
};

pub fn api_services() -> Scope {
    web::scope("/api").service(block_services())
}

fn block_services() -> Scope {
    web::scope("/block")
        .service(block_info_by_height)
        .service(latest_block_info)
        .service(block_result_by_height)
        .service(block_result)
}
