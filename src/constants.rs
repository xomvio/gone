use std::time::Duration;

/// Default value for the `Server` HTTP header.
pub const DEFAULT_SERVER_NAME: &str = "nginx";

/// Default `Content-Type` HTTP header.
pub const DEFAULT_CONTENT_TYPE: &str = "text/plain";

/// Maximum allowed size for an incoming HTTP request (headers only).
pub const MAX_REQUEST_SIZE: usize = 16_384; // 16 KB

/// Time limit for receiving a complete HTTP request (slowloris protection).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Minimum allowed port number.
pub const MIN_PORT: u16 = 1024;
