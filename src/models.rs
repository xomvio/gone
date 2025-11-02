use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub port: Option<String>,
    pub content_type: Option<String>,
    pub server_name: Option<String>,
    pub max_visits: Option<u32>,
    pub allowed_methods: Option<Vec<String>>,
    pub from_file: Option<String>,
    pub text: Option<String>,
    pub endpoint: Option<String>,
    pub output: Option<String>,
    pub blacklist: Option<Vec<String>>,
    pub whitelist: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct Visit {
    pub datetime: String,
    pub ip: String,
    pub endpoint: String,
    pub method: String,
    pub version: String,
}

pub struct Visitor {
    pub visits: Vec<Visit>,
    pub blocked: bool,
}

impl Visitor {
    pub fn check(&mut self, _config: &Config) -> bool {
        if self.blocked {
            return true;
        }

        let mut blocked = false;
        if self.visits.len() > 3 {
            blocked = true;
        }

        let last_visit = match self.visits.last() {
            Some(visit) => visit,
            None => return false, // No visits yet, not blocked
        };
        
        if last_visit.method == "POST" {
            blocked = true;
        }

        self.blocked = blocked;
        blocked
    }
}
