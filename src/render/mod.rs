mod asset_block;
mod background_command_event;
mod code_block;
mod command_output;
mod command_result_block;
mod edit_diff;
mod error;
mod live_tool_status;
mod markdown;
mod markdown_blocks;
mod markdown_inline;
mod permission;
mod session_summary;
#[cfg(test)]
mod session_summary_tests;
mod status_style;
mod stream;
mod stream_config;
mod stream_cursor;
mod stream_output;
mod stream_summary;
mod stream_text;
mod stream_tool_status;
mod streaming_asset_block;
mod streaming_command_block;
mod streaming_replace;
mod style;
mod table;
pub(crate) mod terminal_image;
mod tool_call_blocks;
mod tool_call_preview;
mod tool_event_line;
mod tool_names;
mod tool_view;
pub(crate) mod transcript;
mod wait_spinner;
pub(crate) mod work_status;

pub(crate) use error::write_chat_error;
pub(crate) use permission::{render_permission_event, render_permission_prompt};
pub use session_summary::print_session_summary;
pub use stream::StreamRenderer;
pub use stream_config::{ReasoningDisplayMode, StreamRenderOptions, ToolCallDisplayMode};
pub use stream_output::print_assistant_response;

#[allow(unused_imports)]
pub use stream_output::print_markdown;
