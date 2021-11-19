use crate as cincinnati;

use self::cincinnati::plugins::prelude_plugin_impl::*;
use commons::{CINCINNATI_VERSION, MIN_CINCINNATI_VERSION};

#[derive(Debug, Serialize, Deserialize, SmartDefault)]
#[serde(default)]
pub struct VersionedGraph {
    pub version: i32,
    #[serde(flatten)]
    pub graph: cincinnati::Graph,
}

impl VersionedGraph {
    pub const PLUGIN_NAME: &'static str = "versioned-graph";

    pub fn versioned_graph(io: &InternalIO) -> Fallible<VersionedGraph> {
        let min_version = match CINCINNATI_VERSION.get(*MIN_CINCINNATI_VERSION) {
            Some(version) => *version,
            None => bail!("error parsing minimum cincinnati version"),
        };
        let graph_version: i32 = match io.parameters.get("content_type") {
            None => min_version,
            Some(v) => match CINCINNATI_VERSION.get(v.as_str()) {
                Some(version) => *version,
                None => min_version,
            },
        };
        Ok(VersionedGraph {
            version: graph_version.clone(),
            graph: match graph_version {
                2 => VersionedGraph::v2_graph(&io.graph),
                _ => VersionedGraph::v1_graph(&io.graph),
            },
        })
    }

    fn v1_graph(graph: &cincinnati::Graph) -> cincinnati::Graph {
        cincinnati::Graph {
            dag: graph.dag.clone(),
            conditional_edges: None,
        }
    }

    fn v2_graph(graph: &cincinnati::Graph) -> cincinnati::Graph {
        graph.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate as cincinnati;
    use std::collections::HashMap;

    use super::*;
    use cincinnati::testing::{generate_custom_graph, TestMetadata};
    use commons::testing::init_runtime;
    use commons::MIN_CINCINNATI_VERSION;

    fn get_min_version() -> Fallible<i32> {
        match CINCINNATI_VERSION.get(*MIN_CINCINNATI_VERSION) {
            Some(version) => Ok(*version),
            None => bail!("error parsing minimum cincinnati version"),
        }
    }

    #[test]
    fn min_version_if_missing() -> Fallible<()> {
        let _ = init_runtime()?;

        let input_graph: cincinnati::Graph = {
            let metadata: TestMetadata = vec![(1, [].iter().cloned().collect())];
            generate_custom_graph("image", metadata, None)
        };

        let versioned_graph = VersionedGraph::versioned_graph(&InternalIO {
            graph: input_graph,
            parameters: Default::default(),
        })
        .unwrap();

        assert_eq!(versioned_graph.version, get_min_version().unwrap());
        Ok(())
    }

    #[test]
    fn ensure_min_on_unsupported() -> Fallible<()> {
        let _ = init_runtime()?;

        let input_graph: cincinnati::Graph = {
            let metadata: TestMetadata = vec![(1, [].iter().cloned().collect())];
            generate_custom_graph("image", metadata, None)
        };

        let mut plugin_params: HashMap<String, String> = HashMap::new();
        plugin_params.insert(String::from("version"), "application/json".to_string());

        let versioned_graph = VersionedGraph::versioned_graph(&InternalIO {
            graph: input_graph,
            parameters: plugin_params,
        })
        .unwrap();

        assert_eq!(versioned_graph.version, get_min_version().unwrap());
        Ok(())
    }

    #[test]
    fn ensure_version_1() -> Fallible<()> {
        let _ = init_runtime()?;

        let input_graph: cincinnati::Graph = {
            let metadata: TestMetadata = vec![(1, [].iter().cloned().collect())];
            generate_custom_graph("image", metadata, None)
        };

        let mut plugin_params: HashMap<String, String> = HashMap::new();
        plugin_params.insert(String::from("version"), MIN_CINCINNATI_VERSION.to_string());

        let versioned_graph = VersionedGraph::versioned_graph(&InternalIO {
            graph: input_graph,
            parameters: plugin_params,
        })
        .unwrap();

        assert_eq!(versioned_graph.version, 1);
        Ok(())
    }
}
