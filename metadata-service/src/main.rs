use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use integrity_common::Baseline;
use sled::Db;
use std::sync::Arc;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "metadata-service")]
#[command(about = "Golden Image Integrity Metadata Service", long_about = None)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value = "8080")]
    port: u16,

    #[arg(long, default_value = "./metadata-db")]
    db_path: String,
}



struct AppState {
    db: Arc<Db>,
}

async fn store_baseline(
    baseline: web::Json<Baseline>,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let baseline = baseline.into_inner();
    let image_id = baseline.image_id.clone();

    info!("Storing baseline for image: {}", image_id);

    let serialized = serde_json::to_vec(&baseline)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    data.db
        .insert(image_id.as_bytes(), serialized)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    data.db
        .flush_async()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Created().json(baseline))
}

async fn get_baseline(
    image_id: web::Path<String>,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let image_id = image_id.into_inner();

    info!("Retrieving baseline for image: {}", image_id);

    let serialized = data.db
        .get(image_id.as_bytes())
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Baseline not found: {}", image_id)))?;

    let baseline: Baseline = serde_json::from_slice(&serialized)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok().json(baseline))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Starting metadata service on {}:{}", args.host, args.port);
    info!("Using database at: {}", args.db_path);

    let db = sled::open(&args.db_path)
        .expect("Failed to open database");

    let app_state = web::Data::new(AppState {
        db: Arc::new(db),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(
                web::scope("/baselines")
                    .route("", web::post().to(store_baseline))
                    .route("/{image_id}", web::get().to(get_baseline))
            )
    })
    .bind((args.host, args.port))?
    .run()
    .await
}
