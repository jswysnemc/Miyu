mod budget;
mod estimate;
mod manual;
mod model;
mod projection_budget;
mod prompt;
mod selector;
mod storage;
mod store;

pub use budget::should_compact_for_context_chars;
pub use estimate::{
    estimate_chat_messages_chars, estimate_state_context_chars, estimate_turn_context_chars,
};
pub use manual::select_manual_compaction;
pub use model::{CompactionRequest, CompactionSummary};
pub use prompt::summary_context_message;
pub use selector::select_compaction_after_context_trigger;
pub use storage::{clear_summary, load_summary, save_summary};
pub use store::CompactionApplyOutcome;
