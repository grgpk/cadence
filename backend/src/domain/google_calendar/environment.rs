//! Mirrors `app/src/utils/environment.rs`. Hand-rolled (no strum dependency here for a 2-variant enum).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Environment {
    #[default]
    Production,
    Local,
}

impl Environment {
    pub fn detect() -> Self {
        match std::env::var("ENVIRONMENT") {
            Ok(v) if v.eq_ignore_ascii_case("local") => Self::Local,
            _ => Self::Production,
        }
    }
}
