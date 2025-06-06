mod common;
mod pg;

use axum::{
    extract::Query, http::StatusCode, response::IntoResponse, routing::get, Router,
};
use dotenv::dotenv;
use rand::rng;
use yarte::Template;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(not(feature = "simd-json"))]
use axum::Json;
#[cfg(feature = "simd-json")]
use common::simd_json::Json;

mod server;

use common::{
    get_env, random_id,
    utils::{parse_params, Params, Utf8Html},
};
use pg::database::{DatabaseConnection, PgConnection};
use pg::models::Fortune;

#[derive(Template)]
#[template(path = "fortunes.html.hbs")]
pub struct FortunesTemplate<'a> {
    pub fortunes: &'a Vec<Fortune>,
}

async fn db(DatabaseConnection(conn): DatabaseConnection) -> impl IntoResponse {
    let id = random_id(&mut rng());
    let world = conn
        .fetch_world_by_id(id)
        .await
        .expect("error loading world");

    (StatusCode::OK, Json(world))
}

async fn queries(
    DatabaseConnection(conn): DatabaseConnection,
    Query(params): Query<Params>,
) -> impl IntoResponse {
    let q = parse_params(params);

    let results = conn
        .fetch_random_worlds(q)
        .await
        .expect("error loading worlds");

    (StatusCode::OK, Json(results))
}

async fn fortunes(DatabaseConnection(conn): DatabaseConnection) -> impl IntoResponse {
    let fortunes: Vec<Fortune> = conn
        .fetch_all_fortunes()
        .await
        .expect("error loading fortunes");

    Utf8Html(
        FortunesTemplate {
            fortunes: &fortunes,
        }
        .call()
        .expect("error rendering template"),
    )
}

async fn updates(
    DatabaseConnection(conn): DatabaseConnection,
    Query(params): Query<Params>,
) -> impl IntoResponse {
    let q = parse_params(params);
    let worlds = conn.update_worlds(q).await.expect("error updating worlds");

    (StatusCode::OK, Json(worlds))
}

fn main() {
    dotenv().ok();
    server::start_tokio(serve_app)
}

async fn serve_app() {
    let database_url: String = get_env("POSTGRES_URL");

    // Create shared database connection
    let pg_connection = PgConnection::connect(database_url).await;

    let app = Router::new()
        .route("/fortunes", get(fortunes))
        .route("/db", get(db))
        .route("/queries", get(queries))
        .route("/updates", get(updates))
        .with_state(pg_connection);

    server::serve_hyper(app, Some(8000)).await
}
