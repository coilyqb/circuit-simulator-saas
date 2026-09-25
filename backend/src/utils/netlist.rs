use serde_json::Value;

use crate::{
    errors::AppError,
    models::{
        circuit::{Circuit, Component, Connection},
        simulation::SimulationConfig,
    },
};

pub fn generate_netlist(
    circuit: &Circuit,
    components: &[Component],
    connections: &[Connection],
    config: Option<&SimulationConfig>,
) -> Result<String, AppError> {
    let mut lines = vec![
        format!("* Circuit Simulator SaaS - {}", circuit.name),
        format!("* Circuit ID: {}", circuit.id),
    ];

    for component in components {
        let nodes = nodes_from_value(&component.nodes)?;
        let spice_line = match component.component_type.as_str() {
            "R" => format_component("R", component.id, &nodes, &component.value, 2)?,
            "L" => format_component("L", component.id, &nodes, &component.value, 2)?,
            "C" => format_component("C", component.id, &nodes, &component.value, 2)?,
            "Diode" => format_component("D", component.id, &nodes, &component.value, 2)?,
            "BJT" => format_component("Q", component.id, &nodes, &component.value, 3)?,
            "CMOS" => format_component("M", component.id, &nodes, &component.value, 4)?,
            "VSource" => format_component("V", component.id, &nodes, &component.value, 2)?,
            "ISource" => format_component("I", component.id, &nodes, &component.value, 2)?,
            other => {
                return Err(AppError::Validation(format!(
                    "unsupported component type `{other}`"
                )));
            }
        };
        lines.push(spice_line);
    }

    for connection in connections {
        lines.push(format!(
            "* connection {} -> {}",
            connection.from_node, connection.to_node
        ));
    }

    if let Some(config) = config {
        lines.push(analysis_directive(config)?);
    } else {
        lines.push(".OP".to_owned());
    }

    lines.push(".END".to_owned());

    Ok(lines.join("\n"))
}

fn nodes_from_value(value: &Value) -> Result<Vec<String>, AppError> {
    let array = value
        .as_array()
        .ok_or_else(|| AppError::Validation("component nodes must be a JSON array".to_owned()))?;

    let nodes = array
        .iter()
        .map(|node| {
            node.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                AppError::Validation("component node values must be strings".to_owned())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    if nodes.is_empty() {
        return Err(AppError::Validation(
            "component must provide at least one node".to_owned(),
        ));
    }

    Ok(nodes)
}

fn format_component(
    prefix: &str,
    id: i64,
    nodes: &[String],
    value: &str,
    expected_nodes: usize,
) -> Result<String, AppError> {
    if nodes.len() < expected_nodes {
        return Err(AppError::Validation(format!(
            "component `{prefix}{id}` requires at least {expected_nodes} nodes"
        )));
    }

    Ok(format!("{prefix}{id} {} {value}", nodes.join(" ")))
}

fn analysis_directive(config: &SimulationConfig) -> Result<String, AppError> {
    let parameters = &config.parameters_json;

    match config.analysis_type.as_str() {
        "dc" => Ok(".OP".to_owned()),
        "transient" => Ok(format!(
            ".TRAN {} {}",
            json_parameter(parameters, "step", "1u")?,
            json_parameter(parameters, "stop", "1m")?
        )),
        "ac" => Ok(format!(
            ".AC {} {} {} {}",
            json_parameter(parameters, "sweep_type", "DEC")?,
            json_parameter(parameters, "points", "10")?,
            json_parameter(parameters, "start_frequency", "1")?,
            json_parameter(parameters, "stop_frequency", "1e6")?
        )),
        "sweep" => Ok(format!(
            ".DC {} {} {} {}",
            json_parameter(parameters, "source", "V1")?,
            json_parameter(parameters, "start", "0")?,
            json_parameter(parameters, "stop", "5")?,
            json_parameter(parameters, "step", "0.1")?
        )),
        other => Err(AppError::Validation(format!(
            "unsupported analysis type `{other}`"
        ))),
    }
}

fn json_parameter<'a>(
    parameters: &'a Value,
    key: &str,
    default: &'a str,
) -> Result<String, AppError> {
    match parameters.get(key) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(Value::Number(value)) => Ok(value.to_string()),
        Some(Value::Bool(value)) => Ok(value.to_string()),
        Some(Value::Null) | None => Ok(default.to_owned()),
        Some(_) => Err(AppError::Validation(format!(
            "analysis parameter `{key}` must be a string, number, or boolean"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use super::generate_netlist;
    use crate::models::{
        circuit::{Circuit, Component, Connection},
        simulation::SimulationConfig,
    };

    #[test]
    fn generates_spice_netlist_for_transient_simulation() {
        let circuit = Circuit {
            id: 1,
            user_id: 7,
            name: "RC Filter".into(),
            description: None,
            netlist_json: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let components = vec![
            Component {
                id: 1,
                circuit_id: 1,
                component_type: "R".into(),
                value: "1k".into(),
                nodes: json!(["in", "out"]),
                created_at: Utc::now(),
            },
            Component {
                id: 2,
                circuit_id: 1,
                component_type: "C".into(),
                value: "10n".into(),
                nodes: json!(["out", "0"]),
                created_at: Utc::now(),
            },
        ];
        let connections = vec![Connection {
            id: 1,
            circuit_id: 1,
            from_node: "in".into(),
            to_node: "out".into(),
            created_at: Utc::now(),
        }];
        let config = SimulationConfig {
            id: 1,
            simulation_id: 1,
            analysis_type: "transient".into(),
            parameters_json: json!({
                "step": "1u",
                "stop": "10m"
            }),
            created_at: Utc::now(),
        };

        let netlist = generate_netlist(&circuit, &components, &connections, Some(&config))
            .expect("netlist");

        assert!(netlist.contains("R1 in out 1k"));
        assert!(netlist.contains(".TRAN 1u 10m"));
        assert!(netlist.ends_with(".END"));
    }
}
