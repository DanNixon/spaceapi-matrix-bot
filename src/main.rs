mod api;
mod error;
mod matrix;
mod mqtt;

use clap::Parser;
use matrix_sdk::ruma::OwnedRoomId;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about)]
pub(crate) struct Cli {
    /// Matrix username
    #[clap(value_parser, long)]
    matrix_username: String,

    /// Matrix password
    #[clap(value_parser, long, env)]
    matrix_password: String,

    /// Matrix storage directory
    #[clap(value_parser, long)]
    matrix_storage: PathBuf,

    /// Room(s) to announce open status changes in
    #[clap(value_parser, long)]
    open_announcement_room: Vec<OwnedRoomId>,

    /// SpaceAPI endpoint
    #[clap(value_parser, long)]
    spaceapi_url: String,

    /// MQTT broker address
    #[clap(value_parser, long)]
    mqtt_broker: String,

    /// MQTT username
    #[clap(value_parser, long)]
    mqtt_username: String,

    /// MQTT password
    #[clap(value_parser, long, env)]
    mqtt_password: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    let matrix_client = crate::matrix::init(
        &args.matrix_username,
        &args.matrix_password,
        &args.matrix_storage,
        &args.spaceapi_url,
    )
    .await;

    let mqtt_client =
        crate::mqtt::init(&args.mqtt_broker, &args.mqtt_username, &args.mqtt_password).await;

    let mut mqtt_rx = mqtt_client.rx_channel();

    let last_known_state = Arc::new(Mutex::new(
        crate::api::http_get_spaceapi(&args.spaceapi_url)
            .await
            .expect("spaceapi HTTP request should work")
            .state
            .expect("spaceapi payload shoud have a state"),
    ));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                return;
            }
            Ok(event) = mqtt_rx.recv() => {
                crate::mqtt::handle_spaceapi_message(event, matrix_client.clone(), args.open_announcement_room.clone(), last_known_state.clone()).await;
            }
        }
    }
}
