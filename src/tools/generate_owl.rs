use crate::mcp::protocol::{McpTool, McpToolCallResult};
use crate::ontology::mcguinness_builder::{McGuinnessBuilder, McGuinnessOntologyInput};
use crate::ontology::serializer::{OntologyFormat, OntologySerializer};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct GenerateOwlOntologyArgs {
    #[serde(flatten)]
    pub input: McGuinnessOntologyInput,
    pub format: Option<OntologyFormat>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateOwlOntologyResponse {
    pub ontology_iri: String,
    pub format: OntologyFormat,
    pub class_count: usize,
    pub object_property_count: usize,
    pub data_property_count: usize,
    pub individual_count: usize,
    pub axiom_count: usize,
    pub serialized_ontology: String,
}


pub fn tool_definition() -> McpTool {
    McpTool {
        name: "generate_owl_ontology".to_string(),
        description: "Executes the McGuinness 7-Step structuring logic (Steps 4-7: Classes/Hierarchy, Properties, Facets/Restrictions, Instances) and returns a validated, serialized OWL 2 ontology (Functional Syntax, Turtle, RDF/XML) generated via horned-owl.".to_string(),
        input_schema: json!({

            "type": "object",
            "properties": {
                "ontology_iri": {
                    "type": "string",
                    "description": "Base IRI for the generated ontology (e.g. 'http://example.org/iso/12345#')."
                },
                "prefix": {
                    "type": "string",
                    "description": "Optional default prefix string."
                },
                "format": {
                    "type": "string",
                    "enum": ["ofn", "turtle", "rdfxml"],
                    "description": "Output syntax format. Defaults to 'ofn' (OWL Functional Syntax)."
                },
                "classes": {
                    "type": "array",
                    "description": "McGuinness Step 4: Class definitions and subClassOf parent relationships.",
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
                    "description": "McGuinness Step 5-6: Object property relationships, domains, and ranges.",
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
                    "description": "McGuinness Step 5-6: Data property attributes, domains, and XML Schema datatypes.",
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
                    "description": "McGuinness Step 7: Named individual instances.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "class_name": { "type": "string" },
                            "comment": { "type": "string" }
                        },
                        "required": ["name", "class_name"]
                    }
                }
            },
            "required": ["ontology_iri", "classes"]
        }),
    }
}

pub fn execute(args: GenerateOwlOntologyArgs) -> McpToolCallResult {
    let fmt = args.format.unwrap_or(OntologyFormat::Ofn);

    match McGuinnessBuilder::build(args.input) {
        Ok(built) => match OntologySerializer::serialize(&built.ontology, fmt) {
            Ok(serialized) => {
                let resp = GenerateOwlOntologyResponse {
                    ontology_iri: built.ontology_iri,
                    format: fmt,
                    class_count: built.class_count,
                    object_property_count: built.object_property_count,
                    data_property_count: built.data_property_count,
                    individual_count: built.individual_count,
                    axiom_count: built.axiom_count,
                    serialized_ontology: serialized,
                };
                match serde_json::to_string_pretty(&resp) {
                    Ok(json_str) => McpToolCallResult::text(json_str),
                    Err(e) => McpToolCallResult::error(format!("Failed to serialize response: {}", e)),
                }
            }
            Err(e) => McpToolCallResult::error(format!("Ontology serialization failed: {:#}", e)),
        },
        Err(e) => McpToolCallResult::error(format!("Ontology construction failed: {:#}", e)),
    }
}
