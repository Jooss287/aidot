pub mod claude_code;
pub mod copilot;
pub mod cursor;
pub mod detector;
pub mod traits;

pub use detector::detect_tools;
pub use traits::{ConflictMode, ToolAdapter};
