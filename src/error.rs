use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
}

impl Error {
    pub(crate) fn as_matrix_message(&self) -> RoomMessageEventContent {
        RoomMessageEventContent::text_plain(format!("Something has gone wrong... ({self})"))
    }
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
