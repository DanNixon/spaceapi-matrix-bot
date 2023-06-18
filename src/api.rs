use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;

pub(crate) async fn http_get_spaceapi(endpoint: &str) -> crate::error::Result<spaceapi::Status> {
    Ok(reqwest::get(endpoint).await?.json().await?)
}

pub(crate) fn generate_message_from_status(status: &spaceapi::Status) -> RoomMessageEventContent {
    match &status.state {
        Some(state) => {
            let open_text = if state
                .open
                .expect("if someone provides state they should have provided state.open")
            {
                "open"
            } else {
                "closed"
            };

            let message_text = match &state.message {
                Some(msg) => format!(" ({msg})"),
                None => "".to_string(),
            };

            RoomMessageEventContent::text_markdown(format!(
                "{0} is **{open_text}**{message_text}",
                status.space
            ))
        }
        None => RoomMessageEventContent::text_plain(format!(
            "I have no idea if {0} is open or not...",
            status.space
        )),
    }
}
