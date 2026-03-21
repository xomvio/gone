#[derive(Clone)]
pub struct Visit {
    pub datetime: String,
    pub ip: String,
    pub endpoint: String,
    pub method: String,
    pub version: String,
}
