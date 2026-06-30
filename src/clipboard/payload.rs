#[derive(Debug, Clone)]
pub enum ClipboardPayload {
    Text(String),
    ImageDataUrl(String),
}

#[derive(Debug, Clone)]
pub struct ClipboardChatInput {
    pub message: String,
    pub image_url: Option<String>,
}
