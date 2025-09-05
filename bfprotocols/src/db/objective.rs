use crate::cfg::Deployable;
use dcso3::{atomic_id, String};
use serde_derive::{Deserialize, Serialize};

atomic_id!(ObjectiveId);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectiveKind {
    Airbase,
    Fob,
    Logistics,
    Farp {
        spec: Deployable,
        pad_template: String,
        #[serde(default)]
        mobile: bool,
    },
}

impl ObjectiveKind {
    pub fn is_airbase(&self) -> bool {
        match self {
            Self::Airbase => true,
            Self::Farp { .. } | Self::Fob | Self::Logistics => false,
        }
    }

    pub fn is_farp(&self) -> bool {
        match self {
            Self::Farp { .. } => true,
            Self::Airbase | Self::Fob | Self::Logistics => false,
        }
    }

    pub fn is_hub(&self) -> bool {
        match self {
            Self::Logistics => true,
            Self::Airbase | Self::Farp { .. } | Self::Fob => false,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Airbase => "Airbase",
            Self::Fob => "FOB",
            Self::Farp { .. } => "FARP",
            Self::Logistics => "Logistics Hub",
        }
    }
}
