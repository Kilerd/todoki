use gotcha::Schematic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schematic, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReportPeriod {
    #[default]
    Today,
    Week,
    Month,
}

impl ReportPeriod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "today" => Some(ReportPeriod::Today),
            "week" => Some(ReportPeriod::Week),
            "month" => Some(ReportPeriod::Month),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct ReportResponse {
    pub period: ReportPeriod,
    pub created_count: i64,
    pub done_count: i64,
    pub archived_count: i64,
    pub state_changes_count: i64,
    pub comments_count: i64,
}
