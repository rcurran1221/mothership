use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, State},
    response::IntoResponse,
    routing::post,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sled::{Config, Db, IVec};
use std::{env, fs, net::SocketAddr, sync::Arc};
use toml::de::from_str;
use tracing::{Level, event, level_filters::LevelFilter, span};
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let config: MothershipConfig = match fs::read_to_string(&args[1]) {
        Err(_) => panic!("cannot read config"),
        Ok(content) => from_str(&content).expect("unable to parse config into struct"),
    };

    let file_appender = rolling::daily("logs", "mothership.log");

    let stdout_layer = fmt::layer().with_target(false).with_level(true);

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .with_level(true)
        .with_thread_ids(true)
        .with_writer(file_appender);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .with(LevelFilter::from_level(Level::INFO))
        .init();

    // Combine layers and initialize the subscriber
    let span = span!(Level::INFO, "mothership");
    let _enter = span.enter();

    let node_id = uuid::Uuid::new_v4().hyphenated().to_string();

    event!(Level::INFO, message = "node id generated", node_id);

    let mothership_db = match Config::new().path("mothership_db").open() {
        Ok(db) => db,
        Err(e) => panic!("unable to open mothership db: {e}"),
    };

    let shared_state = Arc::new(AppState { mothership_db });
    let app = Router::new()
        .route("/register", post(register_node_handler))
        .with_state(shared_state)
        .into_make_service_with_connect_info::<SocketAddr>();

    // Run the server
    let addr = format!("0.0.0.0:{}", config.port);

    event!(
        Level::INFO,
        message = "web server is listening",
        address = addr
    );

    // ok to unwrap here
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct RegisterRequest {
    topic_name: String,
    node_id: String,
    node_port: usize,
}

async fn register_node_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(request): Json<RegisterRequest>,
) -> impl IntoResponse {
    let node_address = format!("{}:{}", addr.ip(), request.node_port);

    let topic_name = request.topic_name;
    let node_id = request.node_id;

    let data = format!("{node_address}|{node_id}");
    match state
        .mothership_db
        .insert(topic_name.clone().into_bytes(), data.clone().into_bytes())
    {
        Ok(previous) => match previous {
            Some(previous) => {
                match to_string(previous) {
                    Some(previous) => {
                        event!(
                            Level::INFO,
                            message = "node location updated",
                            previous,
                            new = data
                        )
                    }
                    None => {}
                };
            }
            None => {}
        },
        Err(_) => {
            event!(
                Level::ERROR,
                message = "failed to insert into mothership db",
                topic_name = topic_name.clone(),
            );
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    event!(
        Level::INFO,
        message = "successfully registered node with mothership",
        node_id,
        node_address,
        topic_name,
    );

    StatusCode::OK
}

fn to_string(input: IVec) -> Option<String> {
    match String::from_utf8(input.to_vec()) {
        Ok(s) => Some(s),
        Err(_) => None,
    }
}

async fn get_topic_node_info(
    Path(topic_name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let node_data = match state.mothership_db.get(topic_name.clone().into_bytes()) {
        Ok(o) => match o {
            Some(node_data) => match to_string(node_data) {
                Some(node_data) => node_data,
                None => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({})));
                }
            },
            None => return (StatusCode::BAD_REQUEST, Json(json!({}))),
        },
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({})));
        }
    };

    let mut data_split = node_data.split("|");
    let node_address = match data_split.next() {
        Some(addr) => addr,
        None => {
            event!(
                Level::ERROR,
                message = "bad data in mothership entry",
                topic_name,
                node_data,
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "bad data for mothership entry"})),
            );
        }
    };
    let node_id = match data_split.next() {
        Some(id) => id,
        None => {
            event!(
                Level::ERROR,
                message = "bad data in mothership entry",
                topic_name,
                node_data,
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "bad data for mothership entry"})),
            );
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "node_address": node_address,
            "node_id": node_id,
            "node_topic": topic_name
        })),
    )
}

#[derive(Serialize, Deserialize)]
struct MothershipConfig {
    port: usize,
}

struct AppState {
    mothership_db: Db,
}
