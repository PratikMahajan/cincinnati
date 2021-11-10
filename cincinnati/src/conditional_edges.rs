//use serde::de::{self, Deserialize, Deserializer, Error, MapAccess, Visitor};
//use serde::ser::{Serialize, SerializeStruct, Serializer};

use smart_default::SmartDefault;

/// info
#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone)]
#[serde(default)]
pub struct ConditionalEdge {
    pub edges: Vec<ConditionalUpdateEdge>,
    pub risks: Vec<ConditionalUpdateRisk>,
}

#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone)]
#[serde(default)]
pub struct ConditionalUpdateEdge {
    from: String,
    to: String,
}

#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone)]
#[serde(default)]
pub struct ConditionalUpdateRisk {
    url: String,
    name: String,
    message: String,
    #[serde(rename = "matchingRules")]
    matching_rules: Vec<ClusterCondition>,
}

#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone)]
#[serde(default)]
pub struct ClusterCondition {
    #[serde(rename = "type")]
    condition_type: String,
    promql: PromQLClusterCondition,
}

#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone)]
#[serde(default)]
struct PromQLClusterCondition {
    promql: String,
}
