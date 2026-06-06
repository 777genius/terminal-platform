use crate::dto::{prelude::*, *};

use super::ids::parse_pane_id;

impl TryFrom<&NodeSubscriptionSpec> for SubscriptionSpec {
    type Error = ProtocolError;

    fn try_from(value: &NodeSubscriptionSpec) -> Result<Self, Self::Error> {
        Ok(match value {
            NodeSubscriptionSpec::SessionTopology => Self::SessionTopology,
            NodeSubscriptionSpec::PaneSurface { pane_id } => {
                Self::PaneSurface { pane_id: parse_pane_id(pane_id)? }
            }
        })
    }
}
