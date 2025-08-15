use serde::{Deserialize, Serialize};
use std::{env, fs};
use toml::de::from_str;
use tracing::{Level, event, level_filters::LevelFilter, span};
use tracing_appender::rolling;
use tracing_subscriber::fmt;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    println!("Hello, world!");
    let args: Vec<String> = env::args().collect();
    let config: MothershipConfig = match fs::read_to_string(&args[1]) {
        Err(e) => panic!("cannot read config"),
        Ok(content) => from_str(&content).expect("unable to parse config into struct"),
    };

    let file_appender = rolling::daily("logs", "bob_ka.log");

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
    // input:
    // which port should i run on?
    //
    //
    // apis:
    //   register:
    //    node address, node topics, node name
    //     store in sled db, keyed on node topic?
    //   topic_info:
    //    topic name in, node address out
    //    get info from sled db
    //
    // client then goes and talks directly to bob-ka node it cares about
    //
    // if location of node changes, or topic moves from that node?
    // how does client refresh its knowledge of where the node is?
    // this is likely a future problem, don't want to get to bogged down here
}

#[derive(Serialize, Deserialize)]
struct MothershipConfig {
    port: usize,
}
