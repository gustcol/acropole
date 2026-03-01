use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use clap::Parser;
use integrity_common::{
    AgentHealth, AgentInfo, Alert, AlertSeverity, Baseline, DashboardSummary, Heartbeat, PostAlert,
    RegisterAgent, RegisterAgentResponse,
};
use sled::Db;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

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
    agents: Arc<sled::Tree>,
    heartbeats: Arc<sled::Tree>,
    alerts: Arc<sled::Tree>,
}

// --- Baselines ---

async fn store_baseline(
    baseline: web::Json<Baseline>,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let baseline = baseline.into_inner();
    let image_id = baseline.image_id.clone();

    info!("Storing baseline for image: {}", image_id);

    let serialized = serde_json::to_vec(&baseline)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    data.db
        .insert(image_id.as_bytes(), serialized)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    data.db
        .flush_async()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

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
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Baseline not found: {}", image_id)))?;

    let baseline: Baseline = serde_json::from_slice(&serialized)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(baseline))
}

async fn list_baselines(data: web::Data<AppState>) -> actix_web::Result<impl Responder> {
    use serde::Serialize;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct BaselineListItem {
        image_id: String,
        timestamp: String,
        file_count: usize,
    }

    let mut items = Vec::new();
    for result in data.db.iter() {
        let (_key, value) = result.map_err(actix_web::error::ErrorInternalServerError)?;
        if let Ok(baseline) = serde_json::from_slice::<Baseline>(&value) {
            items.push(BaselineListItem {
                image_id: baseline.image_id,
                timestamp: baseline.timestamp,
                file_count: baseline.entries.len(),
            });
        }
    }

    Ok(HttpResponse::Ok().json(items))
}

// --- Agents ---

async fn register_agent(
    body: web::Json<RegisterAgent>,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let req = body.into_inner();
    let agent_id = Uuid::new_v4().to_string();

    let agent = AgentInfo {
        id: agent_id.clone(),
        hostname: req.hostname,
        ip_address: req.ip_address,
        status: AgentHealth::Healthy,
        last_heartbeat: None,
        alert_count: 0,
        image_id: req.image_id,
    };

    let serialized = serde_json::to_vec(&agent)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    data.agents
        .insert(agent_id.as_bytes(), serialized)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    info!("Registered new agent: {}", agent_id);

    Ok(HttpResponse::Created().json(RegisterAgentResponse { agent_id }))
}

async fn receive_heartbeat(
    body: web::Json<Heartbeat>,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let heartbeat = body.into_inner();
    let agent_id = heartbeat.agent_id.clone();
    let timestamp_millis = heartbeat.timestamp.timestamp_millis();

    let key = format!("{}:{}", agent_id, timestamp_millis);
    let serialized = serde_json::to_vec(&heartbeat)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    data.heartbeats
        .insert(key.as_bytes(), serialized)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // Update the agent's last_heartbeat and status
    if let Some(agent_bytes) = data.agents
        .get(agent_id.as_bytes())
        .map_err(actix_web::error::ErrorInternalServerError)?
    {
        let mut agent: AgentInfo = serde_json::from_slice(&agent_bytes)
            .map_err(actix_web::error::ErrorInternalServerError)?;
        agent.last_heartbeat = Some(heartbeat.timestamp);
        agent.status = heartbeat.status;

        let updated = serde_json::to_vec(&agent)
            .map_err(actix_web::error::ErrorInternalServerError)?;
        data.agents
            .insert(agent_id.as_bytes(), updated)
            .map_err(actix_web::error::ErrorInternalServerError)?;
    }

    info!("Received heartbeat from agent: {}", agent_id);

    Ok(HttpResponse::Ok().finish())
}

async fn receive_alert(
    body: web::Json<PostAlert>,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let req = body.into_inner();
    let alert_id = Uuid::new_v4().to_string();
    let agent_id = req.agent_id.clone();

    let alert = Alert {
        id: alert_id.clone(),
        agent_id: agent_id.clone(),
        message: req.message,
        timestamp: Utc::now(),
        severity: req.severity.clone(),
    };

    let serialized = serde_json::to_vec(&alert)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    data.alerts
        .insert(alert_id.as_bytes(), serialized)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // Update agent status and alert_count
    if let Some(agent_bytes) = data.agents
        .get(agent_id.as_bytes())
        .map_err(actix_web::error::ErrorInternalServerError)?
    {
        let mut agent: AgentInfo = serde_json::from_slice(&agent_bytes)
            .map_err(actix_web::error::ErrorInternalServerError)?;
        agent.alert_count += 1;
        match req.severity {
            AlertSeverity::Critical => {
                agent.status = AgentHealth::Critical;
            }
            AlertSeverity::Warning => {
                if agent.status == AgentHealth::Healthy {
                    agent.status = AgentHealth::Warning;
                }
            }
            AlertSeverity::Info => {}
        }

        let updated = serde_json::to_vec(&agent)
            .map_err(actix_web::error::ErrorInternalServerError)?;
        data.agents
            .insert(agent_id.as_bytes(), updated)
            .map_err(actix_web::error::ErrorInternalServerError)?;
    }

    info!("Received alert from agent: {}", agent_id);

    Ok(HttpResponse::Created().json(alert))
}

async fn list_agents(data: web::Data<AppState>) -> actix_web::Result<impl Responder> {
    let mut agents = Vec::new();
    for result in data.agents.iter() {
        let (_key, value) = result.map_err(actix_web::error::ErrorInternalServerError)?;
        let agent: AgentInfo = serde_json::from_slice(&value)
            .map_err(actix_web::error::ErrorInternalServerError)?;
        agents.push(agent);
    }
    Ok(HttpResponse::Ok().json(agents))
}

async fn get_agent(
    id: web::Path<String>,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let id = id.into_inner();

    let agent_bytes = data.agents
        .get(id.as_bytes())
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Agent not found: {}", id)))?;

    let agent: AgentInfo = serde_json::from_slice(&agent_bytes)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(agent))
}

async fn get_agent_heartbeats(
    id: web::Path<String>,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let id = id.into_inner();
    let prefix = format!("{}:", id);

    let mut heartbeats: Vec<Heartbeat> = data.heartbeats
        .scan_prefix(prefix.as_bytes())
        .filter_map(|result| {
            let (_key, value) = result.ok()?;
            serde_json::from_slice::<Heartbeat>(&value).ok()
        })
        .collect();

    // Return last 50, most recent first (scan_prefix returns lexicographic order)
    heartbeats.reverse();
    heartbeats.truncate(50);

    Ok(HttpResponse::Ok().json(heartbeats))
}

async fn get_agent_alerts(
    id: web::Path<String>,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let id = id.into_inner();

    let mut alerts: Vec<Alert> = data.alerts
        .iter()
        .filter_map(|result| {
            let (_key, value) = result.ok()?;
            let alert = serde_json::from_slice::<Alert>(&value).ok()?;
            if alert.agent_id == id {
                Some(alert)
            } else {
                None
            }
        })
        .collect();

    // Most recent first
    alerts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(HttpResponse::Ok().json(alerts))
}

// --- Dashboard ---

async fn get_dashboard_summary(data: web::Data<AppState>) -> actix_web::Result<impl Responder> {
    let now = Utc::now();
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc())
        .unwrap_or(now);
    let week_start = today_start - chrono::Duration::days(7);
    let month_start = today_start - chrono::Duration::days(30);

    let mut total_agents: u64 = 0;
    let mut healthy_agents: u64 = 0;
    let mut warning_agents: u64 = 0;
    let mut critical_agents: u64 = 0;

    for result in data.agents.iter() {
        let (_key, value) = result.map_err(actix_web::error::ErrorInternalServerError)?;
        if let Ok(agent) = serde_json::from_slice::<AgentInfo>(&value) {
            total_agents += 1;
            match agent.status {
                AgentHealth::Healthy => healthy_agents += 1,
                AgentHealth::Warning => warning_agents += 1,
                AgentHealth::Critical => critical_agents += 1,
            }
        }
    }

    let mut alerts_today: u64 = 0;
    let mut alerts_this_week: u64 = 0;
    let mut alerts_this_month: u64 = 0;

    for result in data.alerts.iter() {
        let (_key, value) = result.map_err(actix_web::error::ErrorInternalServerError)?;
        if let Ok(alert) = serde_json::from_slice::<Alert>(&value) {
            if alert.timestamp >= today_start {
                alerts_today += 1;
            }
            if alert.timestamp >= week_start {
                alerts_this_week += 1;
            }
            if alert.timestamp >= month_start {
                alerts_this_month += 1;
            }
        }
    }

    let summary = DashboardSummary {
        total_agents,
        healthy_agents,
        warning_agents,
        critical_agents,
        alerts_today,
        alerts_this_week,
        alerts_this_month,
    };

    Ok(HttpResponse::Ok().json(summary))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Starting metadata service on {}:{}", args.host, args.port);
    info!("Using database at: {}", args.db_path);

    let db = sled::open(&args.db_path).expect("Failed to open database");

    let agents_tree = db
        .open_tree("agents")
        .expect("Failed to open agents tree");
    let heartbeats_tree = db
        .open_tree("heartbeats")
        .expect("Failed to open heartbeats tree");
    let alerts_tree = db
        .open_tree("alerts")
        .expect("Failed to open alerts tree");

    let app_state = web::Data::new(AppState {
        db: Arc::new(db),
        agents: Arc::new(agents_tree),
        heartbeats: Arc::new(heartbeats_tree),
        alerts: Arc::new(alerts_tree),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(
                web::scope("/baselines")
                    .route("", web::post().to(store_baseline))
                    .route("", web::get().to(list_baselines))
                    .route("/{image_id}", web::get().to(get_baseline)),
            )
            .service(
                web::scope("/agents")
                    .route("/register", web::post().to(register_agent))
                    .route("/heartbeat", web::post().to(receive_heartbeat))
                    .route("/alert", web::post().to(receive_alert))
                    .route("", web::get().to(list_agents))
                    .route("/{id}", web::get().to(get_agent))
                    .route("/{id}/heartbeats", web::get().to(get_agent_heartbeats))
                    .route("/{id}/alerts", web::get().to(get_agent_alerts)),
            )
            .route("/dashboard/summary", web::get().to(get_dashboard_summary))
    })
    .bind((args.host, args.port))?
    .run()
    .await
}
