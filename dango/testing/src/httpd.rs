// use {
//     actix_web::{
//         body::MessageBody,
//         dev::{ServiceFactory, ServiceRequest, ServiceResponse},
//         web::{self, ServiceConfig},
//         App, HttpResponse,
//     },
//     dango_httpd::{
//         graphql::build_schema,
//         server::{build_actix_app, config_app},
//     },
//     grug::{GraphQLCustomRequest, GraphQLCustomResponse, GraphQLResponse},
//     indexer_httpd::{context::Context, server::build_actix_app_with_config},
// };

// pub fn build_app_service(
//     app_ctx: Context,
// ) -> App<
//     impl ServiceFactory<
//         ServiceRequest,
//         Response = ServiceResponse<impl MessageBody>,
//         Config = (),
//         InitError = (),
//         Error = actix_web::Error,
//     >,
// > {
//     let graphql_schema = build_schema(app_ctx.clone());

//     build_actix_app_with_config(app_ctx, graphql_schema, |app_ctx, graphql_schema| {
//         config_app(app_ctx, graphql_schema)
//     })
// }

// pub fn config_app<G>(app_ctx: Context, graphql_schema: G) -> Box<dyn Fn(&mut ServiceConfig)>
// where
//     G: Clone + 'static,
// {
//     Box::new(move |cfg: &mut ServiceConfig| {
//         cfg.service(indexer_httpd::routes::index::index)
//             .service(indexer_httpd::routes::graphql::graphql_route())
//             .default_service(web::to(HttpResponse::NotFound))
//             .app_data(web::Data::new(app_ctx.clone()))
//             .app_data(web::Data::new(graphql_schema.clone()));
//     })
// }

// pub async fn call_graphql<R>(
//     app: App<
//         impl ServiceFactory<
//                 ServiceRequest,
//                 Response = ServiceResponse<impl MessageBody>,
//                 Config = (),
//                 InitError = (),
//                 Error = actix_web::Error,
//             > + 'static,
//     >,
//     request_body: GraphQLCustomRequest<'_>,
// ) -> Result<GraphQLCustomResponse<R>, anyhow::Error>
// where
//     R: serde::de::DeserializeOwned,
//     //     S: async_graphql::ObjectType + Default + 'static + Clone,
// {
//     // let graphql_schema = build_schema(app_ctx.clone());

//     let app = actix_web::test::init_service(app).await;

//     let request = actix_web::test::TestRequest::post()
//         .uri("/graphql")
//         .set_json(&request_body)
//         .to_request();

//     let graphql_response = actix_web::test::call_and_read_body(&app, request).await;

//     // When I need to debug the response
//     println!("text response: \n{:#?}", graphql_response);

//     let mut graphql_response: GraphQLResponse = serde_json::from_slice(&graphql_response)?;

//     // When I need to debug the response
//     println!("GraphQLResponse: {:#?}", graphql_response);

//     if let Some(data) = graphql_response.data.remove(request_body.name) {
//         Ok(GraphQLCustomResponse {
//             data: serde_json::from_value(data)?,
//             errors: graphql_response.errors,
//         })
//     } else {
//         Err(anyhow::anyhow!(
//             "Can't find {} in response",
//             request_body.name
//         ))
//     }
// }
