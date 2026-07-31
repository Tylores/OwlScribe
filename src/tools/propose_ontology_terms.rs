use crate::mcp::protocol::{McpTool, McpToolCallResult};
use crate::ontology::mcguinness_builder::{
    ClassDefinition, ClassMapping, DataPropertyDefinition, IndividualDefinition,
    ObjectPropertyDefinition, PropertyMapping,
};
use crate::ontology::staging::STAGED_INVENTORY;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct ProposeOntologyTermsArgs {
    pub section: Option<String>,
    pub classes: Option<Vec<ClassDefinition>>,
    pub object_properties: Option<Vec<ObjectPropertyDefinition>>,
    pub data_properties: Option<Vec<DataPropertyDefinition>>,
    pub individuals: Option<Vec<IndividualDefinition>>,
    pub class_mappings: Option<Vec<ClassMapping>>,
    pub property_mappings: Option<Vec<PropertyMapping>>,
    pub saref_patterns: Option<Vec<String>>,
    pub clear_staging: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ProposeOntologyTermsResponse {
    pub status: String,
    pub section: String,
    pub total_staged_classes: usize,
    pub total_staged_object_properties: usize,
    pub total_staged_data_properties: usize,
    pub total_staged_individuals: usize,
    pub total_staged_class_mappings: usize,
    pub total_staged_saref_patterns: usize,
    pub summary_markdown: String,
    pub validation_warnings: Vec<String>,
}

pub fn tool_definition() -> McpTool {
    McpTool {
        name: "propose_ontology_terms".to_string(),
        description: "Phase 3: Agent-facing tool to stage, classify (owl:Class, owl:ObjectProperty, owl:DatatypeProperty), and validate discovered ontology terms, candidate superclasses, base ontology mappings, and SAREF design patterns section-by-section prior to final serialization.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "section": {
                    "type": "string",
                    "description": "The section name or title currently being processed (e.g., 'Section 3: Terms and Definitions')."
                },
                "classes": {
                    "type": "array",
                    "description": "Proposed owl:Class definitions for this section.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "parent_class": { "type": "string" },
                            "comment": { "type": "string" }
                        },
                        "required": ["name"]
                    }
                },
                "object_properties": {
                    "type": "array",
                    "description": "Proposed owl:ObjectProperty definitions for this section.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "domain": { "type": "string" },
                            "range": { "type": "string" },
                            "comment": { "type": "string" }
                        },
                        "required": ["name"]
                    }
                },
                "data_properties": {
                    "type": "array",
                    "description": "Proposed owl:DatatypeProperty definitions for this section.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "domain": { "type": "string" },
                            "range": { "type": "string" },
                            "comment": { "type": "string" }
                        },
                        "required": ["name"]
                    }
                },
                "individuals": {
                    "type": "array",
                    "description": "Proposed named individual instances.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "class_name": { "type": "string" },
                            "comment": { "type": "string" }
                        },
                        "required": ["name", "class_name"]
                    }
                },
                "class_mappings": {
                    "type": "array",
                    "description": "Class mappings connecting candidate terms to base ontologies.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "term": { "type": "string" },
                            "target_iri": { "type": "string" },
                            "mapping_type": { "type": "string", "enum": ["equivalentClass", "subClassOf"] }
                        },
                        "required": ["term", "target_iri"]
                    }
                },
                "saref_patterns": {
                    "type": "array",
                    "description": "List of ETSI SAREF Design Patterns to apply (e.g. ['feature_of_interest', 'measurement', 'command_function', 'system_topology', 'state_commodity']).",
                    "items": { "type": "string" }
                },
                "clear_staging": {
                    "type": "boolean",
                    "description": "Optional flag to reset and clear all staged terms before adding."
                }
            }
        }),
    }
}

pub fn execute(args: ProposeOntologyTermsArgs) -> McpToolCallResult {
    let mut inventory = match STAGED_INVENTORY.lock() {
        Ok(guard) => guard,
        Err(e) => return McpToolCallResult::error(format!("Staging lock error: {}", e)),
    };

    if args.clear_staging.unwrap_or(false) {
        inventory.clear();
    }

    let sec_name = args.section.unwrap_or_else(|| "General Staging".to_string());
    if !inventory.staged_sections.contains(&sec_name) {
        inventory.staged_sections.push(sec_name.clone());
    }

    if let Some(classes) = args.classes {
        inventory.add_classes(classes);
    }
    if let Some(ops) = args.object_properties {
        inventory.add_object_properties(ops);
    }
    if let Some(dps) = args.data_properties {
        inventory.add_data_properties(dps);
    }
    if let Some(indivs) = args.individuals {
        inventory.add_individuals(indivs);
    }
    if let Some(cm) = args.class_mappings {
        inventory.add_class_mappings(cm);
    }
    if let Some(pm) = args.property_mappings {
        inventory.add_property_mappings(pm);
    }
    if let Some(sp) = args.saref_patterns {
        inventory.add_saref_patterns(sp);
    }

    let mut warnings = Vec::new();

    let class_names: Vec<String> = inventory.classes.iter().map(|c| c.name.to_lowercase()).collect();
    for c in &inventory.classes {
        match &c.parent_class {
            Some(parent) if !parent.trim().is_empty() => {
                if !class_names.contains(&parent.to_lowercase()) && !parent.contains(':') {
                    warnings.push(format!("Class '{}' references parent class '{}' which is not defined in staged inventory or base ontology.", c.name, parent));
                }
            }
            _ => {
                warnings.push(format!("Class '{}' has no parent_class specified. It will be an unparented root class unless assigned a superclass.", c.name));
            }
        }
    }

    for op in &inventory.object_properties {
        if op.domain.is_none() {
            warnings.push(format!("ObjectProperty '{}' has no specified domain class.", op.name));
        }
        if op.range.is_none() {
            warnings.push(format!("ObjectProperty '{}' has no specified range class.", op.name));
        }
    }

    let mut summary = String::new();
    summary.push_str(&format!("### Staged Ontology Inventory (Section: {})\n\n", sec_name));
    summary.push_str(&format!("- **Total Classes**: {}\n", inventory.classes.len()));
    summary.push_str(&format!("- **Total Object Properties**: {}\n", inventory.object_properties.len()));
    summary.push_str(&format!("- **Total Data Properties**: {}\n", inventory.data_properties.len()));
    summary.push_str(&format!("- **Total Individuals**: {}\n", inventory.individuals.len()));
    summary.push_str(&format!("- **Total Class Mappings**: {}\n\n", inventory.class_mappings.len()));

    if !inventory.classes.is_empty() {
        summary.push_str("| Term / Class Name | Superclass / Parent | Description / Definition |\n");
        summary.push_str("|-------------------|--------------------|--------------------------|\n");
        for c in &inventory.classes {
            let parent_str = c.parent_class.as_deref().unwrap_or("-");
            let comment_str = c.comment.as_deref().unwrap_or("-");
            summary.push_str(&format!("| `{}` | `{}` | {} |\n", c.name, parent_str, comment_str));
        }
    }

    let resp = ProposeOntologyTermsResponse {
        status: "staged".to_string(),
        section: sec_name,
        total_staged_classes: inventory.classes.len(),
        total_staged_object_properties: inventory.object_properties.len(),
        total_staged_data_properties: inventory.data_properties.len(),
        total_staged_individuals: inventory.individuals.len(),
        total_staged_class_mappings: inventory.class_mappings.len(),
        total_staged_saref_patterns: inventory.saref_patterns.len(),
        summary_markdown: summary,
        validation_warnings: warnings,
    };

    match serde_json::to_string_pretty(&resp) {
        Ok(json_str) => McpToolCallResult::text(json_str),
        Err(e) => McpToolCallResult::error(format!("Failed to serialize staging response: {}", e)),
    }
}
