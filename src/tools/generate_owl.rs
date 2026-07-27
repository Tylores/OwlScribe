use crate::mcp::protocol::{McpTool, McpToolCallResult};
use crate::ontology::mcguinness_builder::{McGuinnessBuilder, McGuinnessOntologyInput};
use crate::ontology::serializer::{OntologyFormat, OntologySerializer};
use crate::ontology::staging::STAGED_INVENTORY;
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
        description: "Phase 4: Executes McGuinness 7-Step structuring logic (Steps 4-7: Classes/Hierarchy, Properties, Facets/Restrictions, Instances), incorporates all section terms staged via propose_ontology_terms, and performs full W3C OWL 2 graph binding against base domain ontologies using horned-owl. Emits serialized ontology in Turtle (default), JSON-LD, OFN, or RDF/XML.".to_string(),
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
                    "enum": ["turtle", "jsonld", "ofn", "rdfxml"],
                    "description": "Output syntax format. Defaults to 'turtle' (Turtle .ttl). Also supports 'jsonld', 'ofn', and 'rdfxml'."
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
                },
                "imports": {
                    "type": "array",
                    "description": "List of base domain ontology IRIs to formally import via owl:imports.",
                    "items": { "type": "string" }
                },
                "base_ontology_path": {
                    "type": "string",
                    "description": "Optional file path to a base ontology (.ofn format) to load and merge into the target graph."
                },
                "base_ontology_content": {
                    "type": "string",
                    "description": "Optional raw OFN content string of a base ontology to merge into the target graph."
                },
                "class_mappings": {
                    "type": "array",
                    "description": "Formal mappings connecting candidate PDF classes to base ontology classes (owl:equivalentClass or rdfs:subClassOf).",
                    "items": {
                        "type": "object",
                        "properties": {
                            "term": { "type": "string" },
                            "target_iri": { "type": "string" },
                            "mapping_type": {
                                "type": "string",
                                "enum": ["equivalentClass", "subClassOf"],
                                "description": "Defaults to 'equivalentClass'."
                            }
                        },
                        "required": ["term", "target_iri"]
                    }
                }
            },
            "required": ["ontology_iri"]
        }),
    }
}

pub fn execute(args: GenerateOwlOntologyArgs) -> McpToolCallResult {
    let fmt = args.format.unwrap_or(OntologyFormat::Ofn);
    let mut input = args.input;

    if let Ok(staged) = STAGED_INVENTORY.lock() {
        for sc in &staged.classes {
            if !input.classes.iter().any(|c| c.name.eq_ignore_ascii_case(&sc.name)) {
                input.classes.push(sc.clone());
            }
        }
        for sop in &staged.object_properties {
            if !input.object_properties.iter().any(|p| p.name.eq_ignore_ascii_case(&sop.name)) {
                input.object_properties.push(sop.clone());
            }
        }
        for sdp in &staged.data_properties {
            if !input.data_properties.iter().any(|p| p.name.eq_ignore_ascii_case(&sdp.name)) {
                input.data_properties.push(sdp.clone());
            }
        }
        for ind in &staged.individuals {
            if !input.individuals.iter().any(|i| i.name.eq_ignore_ascii_case(&ind.name)) {
                input.individuals.push(ind.clone());
            }
        }
        for cm in &staged.class_mappings {
            if !input.class_mappings.iter().any(|m| m.term.eq_ignore_ascii_case(&cm.term)) {
                input.class_mappings.push(cm.clone());
            }
        }
        for pm in &staged.property_mappings {
            if !input.property_mappings.iter().any(|m| m.property_name.eq_ignore_ascii_case(&pm.property_name)) {
                input.property_mappings.push(pm.clone());
            }
        }
    }

    match McGuinnessBuilder::build(input) {
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
