use matrix_sdk::ruma::OwnedRoomId;
use mqtt_channel_client::{
    paho_mqtt::{
        connect_options::ConnectOptionsBuilder, create_options::CreateOptionsBuilder,
        PersistenceType,
    },
    Client, ClientConfig, Event, SubscriptionBuilder,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub(super) async fn init(broker: &str, username: &str, password: &str) -> Client {
    let mqtt_client = Client::new(
        CreateOptionsBuilder::new()
            .server_uri(broker)
            .client_id("spaceapi-matrix-bot")
            .persistence(PersistenceType::None)
            .finalize(),
        ClientConfig::default(),
    )
    .unwrap();

    mqtt_client.subscribe(
        SubscriptionBuilder::default()
            .topic("makerspace/spaceapi".into())
            .build()
            .unwrap(),
    );

    mqtt_client
        .start(
            ConnectOptionsBuilder::new()
                .clean_session(true)
                .automatic_reconnect(Duration::from_secs(1), Duration::from_secs(5))
                .keep_alive_interval(Duration::from_secs(5))
                .user_name(username)
                .password(password)
                .finalize(),
        )
        .await
        .unwrap();

    mqtt_client
}

pub(super) async fn handle_spaceapi_message(
    event: Event,
    matrix_client: matrix_client_boilerplate::Client,
    announcement_rooms: Vec<OwnedRoomId>,
    last_known_state: Arc<Mutex<spaceapi::State>>,
) {
    if let Event::Rx(msg) = event {
        match serde_json::from_slice::<spaceapi::Status>(msg.payload()) {
            Ok(status) => match &status.state {
                Some(state) => {
                    info!("Got SpaceAPI payload via MQTT");

                    let mut last_state = last_known_state.lock().await;
                    if *state != *last_state {
                        info!("SpaceAPI payload has a new state");
                        *last_state = state.clone();

                        let content = crate::api::generate_message_from_status(&status);

                        for room in announcement_rooms {
                            match matrix_client.client().get_joined_room(&room) {
                                Some(room) => {
                                    if let Err(err) = room.send(content.clone(), None).await {
                                        error!(
                                            "Failed to send Matrix message to room {0} ({err})",
                                            room.room_id()
                                        );
                                    }
                                }
                                None => {
                                    error!("Failed to get joined room {room}");
                                }
                            }
                        }
                    } else {
                        debug!("No change in state since last message");
                    }
                }
                None => {
                    warn!("SpaceAPI payload has no state");
                }
            },
            Err(err) => {
                error!("Failed to deserialise SpaceAPI payload from MQTT message ({err})");
            }
        }
    }
}
