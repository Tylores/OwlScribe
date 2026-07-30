use owlscribe::ontology::base_ontology::BaseOntologyLoader;
use owlscribe::ontology::mcguinness_builder::{ClassDefinition, ClassMapping, McGuinnessOntologyInput};
use owlscribe::ontology::serializer::OntologyFormat;
use owlscribe::parser::spec_profile::SpecType;
use owlscribe::parser::term_extractor::TermExtractor;
use owlscribe::tools::generate_owl::{self, GenerateOwlOntologyArgs, GenerateOwlOntologyResponse};

#[test]
fn test_hybrid_base_first_and_full_graph_binding() {
    // Sample base domain ontology (SOSA/SSN subset in OFN)
    let sosa_ofn = r#"Prefix(:=<http://www.w3.org/ns/sosa/>)
Ontology(<http://www.w3.org/ns/sosa/>
Declaration(Class(<http://www.w3.org/ns/sosa/Sensor>))
Declaration(Class(<http://www.w3.org/ns/sosa/Observation>))
Declaration(ObjectProperty(<http://www.w3.org/ns/sosa/madeObservation>))
)"#;

    // 1. Seed Extraction: Parse base OFN into BaseOntologySeed
    let (_base_ont, seed) = BaseOntologyLoader::from_ofn_str(sosa_ofn).unwrap();
    assert_eq!(seed.ontology_iri, "http://www.w3.org/ns/sosa/");
    assert!(seed.top_classes.iter().any(|c| c.name == "Sensor"));

    // 2. Base-First PDF Parsing: PDF uses slightly different terminology ("Sensing Unit")
    let pdf_spec_text = r#"
IEEE Std 2026-IoT
1 Scope
This standard specifies sensing units and observation procedures for IoT nodes.

2 Normative references
ISO/IEC 27000 Information security.

3 Terms and definitions
3.1 Sensor: Device that measures physical attributes.
3.2 Sensing Unit: Component responsible for capturing physical environment state.
"#;

    let parse_result = TermExtractor::parse_raw_text(
        pdf_spec_text,
        "IEEE 2026-IoT",
        Some(SpecType::Ieee),
        Some(0.3),
        Some(&seed),
    )
    .unwrap();

    assert!(parse_result.step2_reuse_references.suggested_base_ontologies.contains(&"http://www.w3.org/ns/sosa/".to_string()));

    let sensing_unit_term = parse_result
        .step3_term_enumeration
        .term_candidates
        .iter()
        .find(|t| t.term == "Sensing Unit" || t.term == "Sensor")
        .expect("Should extract Sensor or Sensing Unit candidate");

    assert!(sensing_unit_term.mapped_base_concept.is_some());
    assert_eq!(
        sensing_unit_term.mapped_base_concept.as_ref().unwrap(),
        "http://www.w3.org/ns/sosa/Sensor"
    );
    assert_eq!(
        sensing_unit_term.mapping_relation.as_ref().unwrap(),
        "equivalentClass"
    );

    // 3. Full Graph Binding (Post-Extraction): Generate OWL with owl:imports and equivalentClass mapping
    let ontology_input = McGuinnessOntologyInput {
        ontology_iri: "http://example.org/ieee2026#".to_string(),
        prefix: Some("ieee2026".to_string()),
        classes: vec![
            ClassDefinition {
                name: "SensingUnit".to_string(),
                parent_class: None,
                comment: Some("Component capturing physical environment state".to_string()),
            },
        ],
        object_properties: vec![],
        data_properties: vec![],
        individuals: vec![],
        imports: vec!["http://www.w3.org/ns/sosa/".to_string()],
        base_ontology_path: None,
        base_ontology_content: Some(sosa_ofn.to_string()),
        class_mappings: vec![ClassMapping {
            term: "SensingUnit".to_string(),
            target_iri: "http://www.w3.org/ns/sosa/Sensor".to_string(),
            mapping_type: "equivalentClass".to_string(),
        }],
        property_mappings: vec![],
        saref_patterns: vec![],
    };

    let gen_args = GenerateOwlOntologyArgs {
        input: ontology_input,
        format: Some(OntologyFormat::Ofn),
    };

    let tool_res = generate_owl::execute(gen_args);
    assert!(tool_res.is_error.is_none());

    let response_text = &tool_res.content[0].text;
    let parsed_resp: GenerateOwlOntologyResponse = serde_json::from_str(response_text).unwrap();

    assert!(parsed_resp.serialized_ontology.contains("Import(<http://www.w3.org/ns/sosa/>)"));
    assert!(parsed_resp.serialized_ontology.contains("EquivalentClasses"));
    assert!(parsed_resp.serialized_ontology.contains("http://www.w3.org/ns/sosa/Sensor"));
}
