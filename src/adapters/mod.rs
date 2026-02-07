pub mod claude_code;
pub mod copilot;
pub mod cursor;
pub mod detector;
pub mod traits;

pub use detector::{all_tools, detect_tools};
pub use traits::{normalize_content, write_with_conflict, ConflictMode, ToolAdapter};
