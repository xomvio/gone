use crate::config::Config;

#[derive(Clone)]
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
    pub fn new() -> Self {
        Self { visits: Vec::new(), blocked: false }
    }

    pub fn check(&mut self, config: &Config) -> bool {
        if self.blocked {
            return true;
        }

        // max_visits = 0 or None means unlimited
        let blocked = match config.security.max_visits {
            None | Some(0) => false,
            Some(max) => self.visits.len() > max as usize,
        };

        self.blocked = blocked;
        blocked
    }
}
