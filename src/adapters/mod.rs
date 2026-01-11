pub mod traits;
pub mod detector;
pub mod claude_code;

pub use traits::ToolAdapter;
pub use detector::{detect_tools, DetectedTool};
