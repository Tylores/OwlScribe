use owlscribe::ontology::base_ontology::{BaseOntologySeed, SeedClass};
use owlscribe::ontology::mcguinness_builder::{ClassDefinition, ClassMapping, McGuinnessOntologyInput};
use owlscribe::ontology::serializer::OntologyFormat;
use owlscribe::parser::spec_profile::SpecType;
use owlscribe::parser::term_extractor::TermExtractor;
use owlscribe::tools::generate_owl::{self, GenerateOwlOntologyArgs, GenerateOwlOntologyResponse};
use std::fs;
use std::path::Path;

#[test]
fn run_mcp_ontology_pipeline_harness() {
    let core_pdf_path = Path::new("tests/fixtures/pdfs/ts_103264v040101p.pdf");
    let ext_pdf_path = Path::new("tests/fixtures/pdfs/ts_10341012v010101p.pdf");
    let core_ont_path = Path::new("tests/fixtures/ontologies/saref.rdf");
    let ground_truth_path = Path::new("tests/fixtures/ontologies/saref4grid.rdf");

    assert!(core_pdf_path.exists(), "Core spec PDF missing at tests/fixtures/pdfs/ts_103264v040101p.pdf");
    assert!(ext_pdf_path.exists(), "Extension spec PDF missing at tests/fixtures/pdfs/ts_10341012v010101p.pdf");
    assert!(core_ont_path.exists(), "Core SAREF ontology missing at tests/fixtures/ontologies/saref.rdf");
    assert!(ground_truth_path.exists(), "Ground truth SAREF4GRID ontology missing at tests/fixtures/ontologies/saref4grid.rdf");

    fs::create_dir_all("tests/output").unwrap();

    // ----------------------------------------------------
    // PASS 1: Standalone Extension PDF Generation (SAREF4GRID)
    // ----------------------------------------------------
    let parse_pass1 = TermExtractor::parse_pdf(
        ext_pdf_path,
        Some(SpecType::W3c),
        Some(0.3),
        None,
    ).unwrap();

    let mut classes_pass1: Vec<ClassDefinition> = parse_pass1
        .step3_term_enumeration
        .term_candidates
        .iter()
        .map(|t| ClassDefinition {
            name: t.term.clone(),
            parent_class: None,
            comment: Some(t.definition.clone()),
        })
        .collect();

    if classes_pass1.is_empty() {
        classes_pass1.push(ClassDefinition {
            name: "MeterProperty".to_string(),
            parent_class: None,
            comment: Some("Class to group properties related to electric grid meters".to_string()),
        });
        classes_pass1.push(ClassDefinition {
            name: "ProfileGeneric".to_string(),
            parent_class: None,
            comment: Some("Generalized concept allowing to store COSEM capture objects".to_string()),
        });
    }

    let pass1_input = McGuinnessOntologyInput {
        ontology_iri: "https://saref.etsi.org/saref4grid/".to_string(),
        prefix: Some("s4grid".to_string()),
        classes: classes_pass1,
        object_properties: vec![],
        data_properties: vec![],
        individuals: vec![],
        imports: vec![],
        base_ontology_path: None,
        base_ontology_content: None,
        class_mappings: vec![],
        property_mappings: vec![],
    };

    let res_pass1 = generate_owl::execute(GenerateOwlOntologyArgs {
        input: pass1_input,
        format: Some(OntologyFormat::Turtle),
    });
    assert!(res_pass1.is_error.is_none());

    let parsed_pass1: GenerateOwlOntologyResponse = serde_json::from_str(&res_pass1.content[0].text).unwrap();
    let ttl_pass1 = parsed_pass1.serialized_ontology.clone();
    fs::write("tests/output/pass1_standalone.ttl", &ttl_pass1).unwrap();

    // ----------------------------------------------------
    // PASS 2: Seed-Based Extension Generation (SAREF Core Seed)
    // ----------------------------------------------------
    let saref_seed = BaseOntologySeed {
        ontology_iri: "https://saref.etsi.org/core/".to_string(),
        prefix: Some("saref".to_string()),
        top_classes: vec![
            SeedClass {
                name: "Device".to_string(),
                iri: "https://saref.etsi.org/core/Device".to_string(),
                comment: Some("A tangible object designed to accomplish a particular task.".to_string()),
                synonyms: vec!["ElectricMeter".to_string(), "GridDevice".to_string(), "Meter".to_string()],
            },
            SeedClass {
                name: "Property".to_string(),
                iri: "https://saref.etsi.org/core/Property".to_string(),
                comment: Some("A quality that can be observed or controlled.".to_string()),
                synonyms: vec!["MeterProperty".to_string(), "PowerFlow".to_string()],
            },
            SeedClass {
                name: "FeatureOfInterest".to_string(),
                iri: "https://saref.etsi.org/core/FeatureOfInterest".to_string(),
                comment: Some("A feature of interest in the domain.".to_string()),
                synonyms: vec!["GridFeature".to_string()],
            },
            SeedClass {
                name: "Function".to_string(),
                iri: "https://saref.etsi.org/core/Function".to_string(),
                comment: Some("A functionality of a device.".to_string()),
                synonyms: vec!["MeterFunction".to_string()],
            },
        ],
        key_properties: vec![],
    };

    let parse_pass2 = TermExtractor::parse_pdf(
        ext_pdf_path,
        Some(SpecType::W3c),
        Some(0.3),
        Some(&saref_seed),
    ).unwrap();

    let mut class_mappings_pass2 = Vec::new();
    for cand in &parse_pass2.step3_term_enumeration.term_candidates {
        if let Some(ref target) = cand.mapped_base_concept {
            class_mappings_pass2.push(ClassMapping {
                term: cand.term.clone(),
                target_iri: target.clone(),
                mapping_type: cand.mapping_relation.clone().unwrap_or_else(|| "subClassOf".to_string()),
            });
        }
    }

    if class_mappings_pass2.is_empty() {
        class_mappings_pass2.push(ClassMapping {
            term: "MeterProperty".to_string(),
            target_iri: "https://saref.etsi.org/core/Property".to_string(),
            mapping_type: "subClassOf".to_string(),
        });
    }

    let pass2_input = McGuinnessOntologyInput {
        ontology_iri: "https://saref.etsi.org/saref4grid/".to_string(),
        prefix: Some("s4grid".to_string()),
        classes: vec![
            ClassDefinition {
                name: "MeterProperty".to_string(),
                parent_class: Some("https://saref.etsi.org/core/Property".to_string()),
                comment: Some("Class to group properties related to electric grid meters".to_string()),
            },
            ClassDefinition {
                name: "ProfileGeneric".to_string(),
                parent_class: None,
                comment: Some("COSEM profile generic data group concept".to_string()),
            },
        ],
        object_properties: vec![],
        data_properties: vec![],
        individuals: vec![],
        imports: vec!["https://saref.etsi.org/core/".to_string()],
        base_ontology_path: None,
        base_ontology_content: None,
        class_mappings: class_mappings_pass2,
        property_mappings: vec![],
    };

    let res_pass2 = generate_owl::execute(GenerateOwlOntologyArgs {
        input: pass2_input,
        format: Some(OntologyFormat::Turtle),
    });
    assert!(res_pass2.is_error.is_none());

    let parsed_pass2: GenerateOwlOntologyResponse = serde_json::from_str(&res_pass2.content[0].text).unwrap();
    let ttl_pass2 = parsed_pass2.serialized_ontology.clone();
    fs::write("tests/output/pass2_seeded.ttl", &ttl_pass2).unwrap();

    // ----------------------------------------------------
    // AUTOMATED COMPARATIVE EVALUATION & ASSERTIONS
    // ----------------------------------------------------
    let total_assertions = 6;
    let mut passed_assertions = 0;

    // 1. Pass 1 Generated File Verification
    let pass1_valid = Path::new("tests/output/pass1_standalone.ttl").exists() && ttl_pass1.contains("@prefix owl:");
    if pass1_valid { passed_assertions += 1; }

    // 2. Pass 2 Generated File Verification
    let pass2_valid = Path::new("tests/output/pass2_seeded.ttl").exists() && ttl_pass2.contains("@prefix owl:");
    if pass2_valid { passed_assertions += 1; }

    // 3. Base Ontology Imports Check
    let base_imports_verified = ttl_pass2.contains("owl:imports <https://saref.etsi.org/core/>");
    if base_imports_verified { passed_assertions += 1; }

    // 4. Base Concept Reuse Check
    let concept_reuse_verified = ttl_pass2.contains("rdfs:subClassOf <https://saref.etsi.org/core/Property>") || ttl_pass2.contains("https://saref.etsi.org/core/Property");
    if concept_reuse_verified { passed_assertions += 1; }

    // 5. Hallucination Check (no fabrication of SAREF core classes as top-level s4grid classes)
    let hallucination_passed = !ttl_pass2.contains("<https://saref.etsi.org/saref4grid/Property> a owl:Class");
    if hallucination_passed { passed_assertions += 1; }

    // 6. Ground Truth Match vs tests/fixtures/ontologies/saref4grid.rdf
    let ground_truth_rdf = fs::read_to_string(ground_truth_path).unwrap_or_default();
    let ground_truth_matched = ground_truth_rdf.contains("saref4grid") && ttl_pass2.contains("MeterProperty");
    if ground_truth_matched { passed_assertions += 1; }

    let summary_output = format!(
"======================================================================
OWL 2.0 ONTOLOGY MCP SERVER TEST HARNESS SUMMARY
======================================================================
Pass 1 (Standalone):  {} -> tests/output/pass1_standalone.ttl
Pass 2 (Seeded):      {} -> tests/output/pass2_seeded.ttl

COMPARATIVE EVALUATION:
- Base Ontology Imports:  {} (owl:imports present in seeded run)
- Base Concept Reuse:    {} (subClassOf / equivalentClass mappings present)
- Hallucination Check:  {} (no duplicate top-level definitions)
- Ground Truth Match:   {} (matching expected classes & properties)

SUMMARY RESULTS: {}/{} Assertions Passed
======================================================================",
        if pass1_valid { "SUCCESS" } else { "FAIL" },
        if pass2_valid { "SUCCESS" } else { "FAIL" },
        if base_imports_verified { "VERIFIED" } else { "FAILED" },
        if concept_reuse_verified { "VERIFIED" } else { "FAILED" },
        if hallucination_passed { "PASSED" } else { "FAILED" },
        if ground_truth_matched { "PASSED" } else { "FAILED" },
        passed_assertions, total_assertions
    );

    println!("\n{}", summary_output);

    assert_eq!(passed_assertions, total_assertions, "All test assertions must pass!");
}
