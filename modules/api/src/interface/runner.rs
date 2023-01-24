use std::sync::Arc;

use crate::AppState;
use aide::{
    axum::ApiRouter,
    openapi::{OpenApi, Tag},
    transform::TransformOpenApi,
};
use axum::{http::StatusCode, Extension};
use docs::docs_routes;
use errors::AppError;
use extractors::Json;
use todos::routes::todo_routes;
use uuid::Uuid;

pub struct Runner {}

impl Runner {
    pub async fn run() {
        aide::gen::on_error(|error| {
            println!("{error}");
        });

        aide::gen::extract_schemas(true);

        let state = AppState::default();

        let mut api = OpenApi::default();

        let app = ApiRouter::new()
            .nest_api_service("/todo", todo_routes(state.clone()))
            .nest_api_service("/docs", docs_routes(state.clone()))
            .finish_api_with(&mut api, api_docs)
            .layer(Extension(Arc::new(api)))
            .with_state(state);

        println!("Example docs are accessible at http://127.0.0.1:3000/docs");

        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    }

    fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
        api.title("Aide axum Open API")
            .summary("An example Todo application")
            .description(include_str!("README.md"))
            .tag(Tag {
                name: "todo".into(),
                description: Some("Todo Management".into()),
                ..Default::default()
            })
            .security_scheme(
                "ApiKey",
                aide::openapi::SecurityScheme::ApiKey {
                    location: aide::openapi::ApiKeyLocation::Header,
                    name: "X-Auth-Key".into(),
                    description: Some("A key that is ignored.".into()),
                    extensions: Default::default(),
                },
            )
            .default_response_with::<Json<AppError>, _>(|res| {
                res.example(AppError {
                    error: "some error happened".to_string(),
                    error_details: None,
                    error_id: Uuid::nil(),
                    // This is not visible.
                    status: StatusCode::IM_A_TEAPOT,
                })
            })
    }
}
