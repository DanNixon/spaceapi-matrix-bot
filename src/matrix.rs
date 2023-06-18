use matrix_client_boilerplate::Client;
use matrix_sdk::{
    event_handler::Ctx,
    room::Room,
    ruma::{
        events::room::{
            member::StrippedRoomMemberEvent,
            message::{MessageType, OriginalSyncRoomMessageEvent},
        },
        OwnedUserId, UserId,
    },
};
use std::{path::Path, time::Duration};
use tracing::{error, info};

pub(super) async fn init(
    username: &str,
    password: &str,
    storage: &Path,
    spaceapi_url: &str,
) -> Client {
    let matrix_client = Client::new(username, password, "spaceapi-matrix-bot", storage)
        .await
        .unwrap();

    matrix_client.initial_sync().await.unwrap();

    matrix_client.client().add_event_handler(handle_invitations);
    matrix_client.client().add_event_handler(on_room_message);
    matrix_client
        .client()
        .add_event_handler_context(MatrixEventHandlerContext {
            bot_user: UserId::parse(username).expect("matrix username should be a valid user ID"),
            spaceapi_url: spaceapi_url.to_string(),
        });

    matrix_client.start_background_sync().await;

    matrix_client
}

async fn handle_invitations(
    room_member: StrippedRoomMemberEvent,
    client: matrix_sdk::Client,
    room: Room,
) {
    if room_member.state_key != client.user_id().unwrap() {
        return;
    }

    if let Room::Invited(room) = room {
        tokio::spawn(async move {
            info!("Autojoining room {}", room.room_id());
            let mut delay = 2;

            while let Err(err) = room.accept_invitation().await {
                error!(
                    "Failed to join room {} ({err:?}), retrying in {delay}s",
                    room.room_id()
                );

                tokio::time::sleep(Duration::from_secs(delay)).await;
                delay *= 2;

                if delay > 3600 {
                    error!("Can't join room {} ({err:?})", room.room_id());
                    break;
                }
            }

            info!("Successfully joined room {}", room.room_id());
        });
    }
}

#[derive(Clone)]
struct MatrixEventHandlerContext {
    bot_user: OwnedUserId,
    spaceapi_url: String,
}

async fn on_room_message(
    event: OriginalSyncRoomMessageEvent,
    room: Room,
    ctx: Ctx<MatrixEventHandlerContext>,
) {
    let room_id = room.room_id().to_owned();

    if let Room::Joined(room) = room {
        // Ignore messages sent from the bot
        // (replies can include the original text, i.e. !space, in the message body)
        if event.sender == ctx.bot_user {
            return;
        }

        let MessageType::Text(text_content) = event.content.msgtype.clone() else {
            return;
        };

        let event = event.clone().into_full_event(room_id);

        if text_content.body.contains("!space") {
            let content = match crate::api::http_get_spaceapi(&ctx.spaceapi_url).await {
                Ok(status) => crate::api::generate_message_from_status(&status),
                Err(err) => err.as_matrix_message(),
            };
            let content = content.make_reply_to(&event);

            room.send(content, None).await.unwrap();
        }

        room.read_receipt(&event.event_id).await.unwrap();
    }
}
