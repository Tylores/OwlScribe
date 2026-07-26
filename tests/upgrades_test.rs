use owlscribe::ontology::base_ontology::BaseOntologyLoader;
use owlscribe::parser::spec_profile::SpecType;
use owlscribe::parser::term_extractor::TermExtractor;
use std::path::Path;

#[test]
fn test_multi_format_base_ontology_ingestion() {
    let rdf_path = Path::new("tests/fixtures/ontologies/saref.rdf");
    let rdf4grid_path = Path::new("tests/fixtures/ontologies/saref4grid.rdf");

    assert!(rdf_path.exists(), "saref.rdf fixture must exist");
    assert!(rdf4grid_path.exists(), "saref4grid.rdf fixture must exist");

    // Load RDF/XML ontologies directly
    let (_saref_ont, saref_seed) = BaseOntologyLoader::from_file(rdf_path)
        .expect("BaseOntologyLoader must support RDF/XML (.rdf) ingestion");
    assert_eq!(saref_seed.ontology_iri, "https://saref.etsi.org/core/");
    assert!(saref_seed.top_classes.iter().any(|c| c.name == "Device" || c.name == "Command" || c.name == "Function"));

    let (_s4g_ont, s4g_seed) = BaseOntologyLoader::from_file(rdf4grid_path)
        .expect("BaseOntologyLoader must support saref4grid.rdf ingestion");
    assert_eq!(s4g_seed.ontology_iri, "https://saref.etsi.org/saref4grid/");
}

#[test]
fn test_noise_filtering_and_section_aware_extraction() {
    let text = r#"
ETSI TS 103 264 V4.1.1 (2026-07)
Part 12: General provisions
Sous-clause 3.1
Scope: This document specifies smart devices.

3 Terms and definitions
3.1 SmartDevice: A physical object capable of communication and sensing.
3.2 Actuator: A device responsible for moving or controlling a mechanism.

Clause 4 Architecture
4.1 System Overview
where the device should report state under normal conditions.
"#;

    let result = TermExtractor::parse_raw_text(
        text,
        "ETSI TS 103 264",
        Some(SpecType::W3c),
        Some(0.3),
        None,
    )
    .unwrap();

    let terms: Vec<String> = result
        .step3_term_enumeration
        .term_candidates
        .iter()
        .map(|t| t.term.clone())
        .collect();

    // Verify noise terms are excluded
    assert!(!terms.contains(&"Sous".to_string()), "Noise word 'Sous' must be filtered out");
    assert!(!terms.contains(&"where".to_string()), "Noise word 'where' must be filtered out");
    assert!(!terms.contains(&"should".to_string()), "Noise word 'should' must be filtered out");
    assert!(!terms.contains(&"Part 12".to_string()), "Header 'Part 12' must be filtered out");

    // Verify normative terms are extracted
    assert!(terms.contains(&"SmartDevice".to_string()), "Normative term 'SmartDevice' must be extracted");
    assert!(terms.contains(&"Actuator".to_string()), "Normative term 'Actuator' must be extracted");
}

#[test]
fn test_automated_relationship_mining_and_alignment_matrix() {
    let rdf_path = Path::new("tests/fixtures/ontologies/saref.rdf");
    let (_saref_ont, saref_seed) = BaseOntologyLoader::from_file(rdf_path).unwrap();

    let text = r#"
ETSI TS 103 410-12 V1.1.1
3 Terms and definitions
3.1 Device: Hardware entity in smart grid.
3.2 Meter: Device that measures energy consumption.
Meter is a subclass of Device.
Meter measures EnergyConsumption.
Voltage is measured in Volts.
"#;

    let result = TermExtractor::parse_raw_text(
        text,
        "SAREF4GRID Spec",
        Some(SpecType::W3c),
        Some(0.3),
        Some(&saref_seed),
    )
    .unwrap();

    // McGuinness Steps 5-6 Relationship Mining assertions
    let mined = result.step5_6_mined_relationships;
    assert!(
        mined.subclass_relations.iter().any(|r| r.sub_class == "Meter" && r.super_class == "Device"),
        "Should mine subclass relation Meter -> Device"
    );
    assert!(
        mined.data_properties.iter().any(|dp| dp.property_name == "Voltage" && dp.range_or_unit == "Volts"),
        "Should mine data property / unit Voltage in Volts"
    );

    // Term Alignment Matrix assertions
    assert!(
        !result.term_alignment_matrix.is_empty(),
        "Term alignment matrix should contain matched seed concepts"
    );
    let device_entry = result
        .term_alignment_matrix
        .iter()
        .find(|e| e.candidate_term == "Device" || e.candidate_term == "Meter")
        .expect("Alignment matrix should contain matched term");

    assert!(device_entry.matched_base_iri.contains("saref.etsi.org/core/"));
    assert!(device_entry.confidence_score > 0.3);
    assert!(!device_entry.suggested_relation.is_empty());
}
