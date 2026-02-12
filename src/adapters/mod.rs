pub mod claude_code;
pub mod common;
pub mod conflict;
pub mod copilot;
pub mod cursor;
pub mod detector;
pub mod helpers;
pub mod traits;

pub use conflict::{write_with_conflict, ConflictMode};
pub use detector::{all_tools, detect_tools};
pub use helpers::normalize_content;
pub use traits::ToolAdapter;
