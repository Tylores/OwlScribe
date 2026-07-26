use owlscribe::ontology::mcguinness_builder::{ClassDefinition, McGuinnessOntologyInput};
use owlscribe::ontology::serializer::OntologyFormat;
use owlscribe::parser::spec_profile::SpecType;
use owlscribe::parser::term_extractor::TermExtractor;
use owlscribe::tools::generate_owl::{self, GenerateOwlOntologyArgs, GenerateOwlOntologyResponse};

#[test]
fn test_end_to_end_parse_and_generate_ontology() {
    let mock_iso_spec = r#"
ISO/IEC 27000:2026
1 Scope
This document specifies information security ontology requirements.

2 Normative references
ISO/IEC 27001 Information technology — Security techniques.

3 Terms and definitions
3.1 Confidentiality: Property that information is not made available or disclosed to unauthorized individuals, entities, or processes.
3.2 Integrity: Property of accuracy and completeness.
"#;

    // Step 1: Parse specification to terms (McGuinness Steps 1-3)
    let parse_result = TermExtractor::parse_raw_text(
        mock_iso_spec,
        "ISO/IEC 27000",
        Some(SpecType::Iso),
        Some(0.5),
        None,
    )
    .unwrap();

    assert_eq!(parse_result.step1_domain_scope.detected_spec_type, SpecType::Iso);
    assert!(parse_result.step3_term_enumeration.total_terms_found >= 2);

    // Step 2: Build McGuinness input (Steps 4-7) using extracted terms
    let classes: Vec<ClassDefinition> = parse_result
        .step3_term_enumeration
        .term_candidates
        .iter()
        .map(|t| ClassDefinition {
            name: t.term.clone(),
            parent_class: Some("SecurityProperty".to_string()),
            comment: Some(t.definition.clone()),
        })
        .collect();

    let ontology_input = McGuinnessOntologyInput {
        ontology_iri: "http://iso.org/ontology/27000#".to_string(),
        prefix: Some("iso27000".to_string()),
        classes,
        object_properties: vec![],
        data_properties: vec![],
        individuals: vec![],
        imports: vec![],
        base_ontology_path: None,
        base_ontology_content: None,
        class_mappings: vec![],
        property_mappings: vec![],
    };

    // Step 3: Execute tool generate_owl_ontology
    let gen_args = GenerateOwlOntologyArgs {
        input: ontology_input,
        format: Some(OntologyFormat::Ofn),
    };

    let tool_res = generate_owl::execute(gen_args);
    assert!(tool_res.is_error.is_none());
    assert!(!tool_res.content.is_empty());

    let response_text = &tool_res.content[0].text;
    let parsed_resp: GenerateOwlOntologyResponse = serde_json::from_str(response_text).unwrap();

    assert_eq!(parsed_resp.ontology_iri, "http://iso.org/ontology/27000#");
    assert!(parsed_resp.class_count >= 2);
    assert!(parsed_resp.serialized_ontology.contains("Confidentiality"));
    assert!(parsed_resp.serialized_ontology.contains("Integrity"));
}
