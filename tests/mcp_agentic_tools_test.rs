use owlscribe::ontology::mcguinness_builder::ClassDefinition;
use owlscribe::ontology::serializer::OntologyFormat;
use owlscribe::parser::spec_profile::SpecType;
use owlscribe::tools::{
    generate_owl::{self, GenerateOwlOntologyArgs, GenerateOwlOntologyResponse},
    get_pdf_sections::{self, GetPdfSectionsArgs},
    propose_ontology_terms::{self, ProposeOntologyTermsArgs},
    read_pdf_section::{self, ReadPdfSectionArgs},
};
use std::path::Path;

#[test]
fn test_agentic_3_phase_parsing_workflow() {
    let pdf_path = Path::new("tests/fixtures/pdfs/ts_10341012v010101p.pdf");
    assert!(pdf_path.exists(), "PDF fixture must exist");

    // PHASE 1: Table of Contents & Section Selection
    let sections_res = get_pdf_sections::execute(GetPdfSectionsArgs {
        pdf_path: pdf_path.to_string_lossy().to_string(),
        spec_type: Some(SpecType::W3c),
    });
    assert!(sections_res.is_error.is_none(), "get_pdf_sections failed: {:?}", sections_res.content);

    let sec_json: serde_json::Value = serde_json::from_str(&sections_res.content[0].text).unwrap();
    let sections = sec_json.get("sections").and_then(|v| v.as_array()).expect("sections array missing");
    assert!(!sections.is_empty(), "PDF sections should not be empty");

    // PHASE 2: Section-by-Section Term Extraction
    let first_sec_id = sections[0].get("id").and_then(|v| v.as_str()).unwrap_or("sec_1");
    let read_res = read_pdf_section::execute(ReadPdfSectionArgs {
        pdf_path: pdf_path.to_string_lossy().to_string(),
        section_id: Some(first_sec_id.to_string()),
        section_title: None,
        page_start: None,
        page_end: None,
        spec_type: Some(SpecType::W3c),
        min_confidence: Some(0.3),
        base_ontology_path: None,
        base_ontology_seed: None,
        saref_patterns: None,
    });
    assert!(read_res.is_error.is_none(), "read_pdf_section failed: {:?}", read_res.content);

    let read_json: serde_json::Value = serde_json::from_str(&read_res.content[0].text).unwrap();
    assert!(read_json.get("text").is_some());

    // PHASE 3: Intermediate Staging & Verification
    let propose_res = propose_ontology_terms::execute(ProposeOntologyTermsArgs {
        section: Some("Section 3: Terms and Definitions".to_string()),
        classes: Some(vec![
            ClassDefinition {
                name: "AgenticDevice".to_string(),
                parent_class: None,
                comment: Some("Test staged class for agentic parsing".to_string()),
            },
            ClassDefinition {
                name: "SmartGridMeter".to_string(),
                parent_class: Some("AgenticDevice".to_string()),
                comment: Some("Subclass of AgenticDevice".to_string()),
            },
        ]),
        object_properties: None,
        data_properties: None,
        individuals: None,
        class_mappings: None,
        property_mappings: None,
        saref_patterns: None,
        clear_staging: Some(true),
    });
    assert!(propose_res.is_error.is_none(), "propose_ontology_terms failed: {:?}", propose_res.content);

    let propose_json: serde_json::Value = serde_json::from_str(&propose_res.content[0].text).unwrap();
    assert_eq!(propose_json.get("total_staged_classes").and_then(|v| v.as_u64()), Some(2));

    // PHASE 4: Horned-OWL Serialization (merging staged terms)
    let gen_res = generate_owl::execute(GenerateOwlOntologyArgs {
        input: owlscribe::ontology::mcguinness_builder::McGuinnessOntologyInput {
            ontology_iri: "https://example.org/agentic/".to_string(),
            prefix: Some("agent".to_string()),
            classes: vec![], // Will merge staged terms!
            object_properties: vec![],
            data_properties: vec![],
            individuals: vec![],
            imports: vec![],
            base_ontology_path: None,
            base_ontology_content: None,
            class_mappings: vec![],
            property_mappings: vec![],
            saref_patterns: vec![],
        },
        format: Some(OntologyFormat::Turtle),
    });
    assert!(gen_res.is_error.is_none(), "generate_owl_ontology failed: {:?}", gen_res.content);

    let gen_json: GenerateOwlOntologyResponse = serde_json::from_str(&gen_res.content[0].text).unwrap();
    assert_eq!(gen_json.class_count, 2);
    assert!(gen_json.serialized_ontology.contains("AgenticDevice"));
    assert!(gen_json.serialized_ontology.contains("SmartGridMeter"));
}
