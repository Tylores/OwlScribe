use owlscribe::ontology::mcguinness_builder::{
    ClassDefinition, McGuinnessBuilder, McGuinnessOntologyInput, ObjectPropertyDefinition,
};
use horned_owl::model::*;

#[test]
fn test_subclass_axiom_generation_and_audit() {
    let input = McGuinnessOntologyInput {
        ontology_iri: "http://example.org/smartgrid#".to_string(),
        prefix: Some("grid".to_string()),
        classes: vec![
            ClassDefinition {
                name: "Device".to_string(),
                parent_class: None,
                comment: Some("Root device concept".to_string()),
            },
            ClassDefinition {
                name: "EndDevice".to_string(),
                parent_class: Some("Device".to_string()),
                comment: Some("Smart end device".to_string()),
            },
            ClassDefinition {
                name: "SmartMeter".to_string(),
                parent_class: Some("EndDevice".to_string()),
                comment: Some("Electric smart meter".to_string()),
            },
        ],
        object_properties: vec![ObjectPropertyDefinition {
            name: "hasSubDevice".to_string(),
            domain: Some("Device".to_string()),
            range: Some("EndDevice".to_string()),
            comment: None,
        }],
        data_properties: vec![],
        individuals: vec![],
        imports: vec![],
        base_ontology_path: None,
        base_ontology_content: None,
        class_mappings: vec![],
        property_mappings: vec![],
        saref_patterns: vec![],
    };

    let result = McGuinnessBuilder::build(input).expect("Build should succeed");

    assert_eq!(result.class_count, 3);
    assert_eq!(result.taxonomy_audit.total_classes, 3);
    assert_eq!(result.taxonomy_audit.subclass_axioms, 2);
    assert_eq!(result.taxonomy_audit.object_property_domain_axioms, 1);
    assert_eq!(result.taxonomy_audit.object_property_range_axioms, 1);

    // "Device" is superclass to EndDevice so it's not orphaned.
    // "EndDevice" is sub to Device and super to SmartMeter.
    // "SmartMeter" is sub to EndDevice.
    // Zero orphaned classes expected!
    assert!(result.taxonomy_audit.orphaned_classes.is_empty(), "Orphaned classes found: {:?}", result.taxonomy_audit.orphaned_classes);
}

#[test]
fn test_orphaned_class_detection() {
    let input = McGuinnessOntologyInput {
        ontology_iri: "http://example.org/test#".to_string(),
        prefix: Some("ex".to_string()),
        classes: vec![
            ClassDefinition {
                name: "ConnectedParent".to_string(),
                parent_class: None,
                comment: None,
            },
            ClassDefinition {
                name: "ConnectedChild".to_string(),
                parent_class: Some("ConnectedParent".to_string()),
                comment: None,
            },
            ClassDefinition {
                name: "StandaloneOrphan".to_string(),
                parent_class: None,
                comment: Some("This class is flat / unparented and has no subclasses".to_string()),
            },
        ],
        object_properties: vec![],
        data_properties: vec![],
        individuals: vec![],
        imports: vec![],
        base_ontology_path: None,
        base_ontology_content: None,
        class_mappings: vec![],
        property_mappings: vec![],
        saref_patterns: vec![],
    };

    let result = McGuinnessBuilder::build(input).expect("Build should succeed");

    assert_eq!(result.taxonomy_audit.total_classes, 3);
    assert_eq!(result.taxonomy_audit.subclass_axioms, 1);
    assert_eq!(result.taxonomy_audit.orphaned_classes.len(), 1);
    assert_eq!(
        result.taxonomy_audit.orphaned_classes[0],
        "http://example.org/test#StandaloneOrphan"
    );
}

#[test]
fn test_functional_axiom_inspection() {
    let input = McGuinnessOntologyInput {
        ontology_iri: "http://example.org/inspection#".to_string(),
        prefix: Some("insp".to_string()),
        classes: vec![
            ClassDefinition {
                name: "ParentNode".to_string(),
                parent_class: None,
                comment: None,
            },
            ClassDefinition {
                name: "ChildNode".to_string(),
                parent_class: Some("ParentNode".to_string()),
                comment: None,
            },
        ],
        object_properties: vec![ObjectPropertyDefinition {
            name: "connectsTo".to_string(),
            domain: Some("ParentNode".to_string()),
            range: Some("ChildNode".to_string()),
            comment: None,
        }],
        data_properties: vec![],
        individuals: vec![],
        imports: vec![],
        base_ontology_path: None,
        base_ontology_content: None,
        class_mappings: vec![],
        property_mappings: vec![],
        saref_patterns: vec![],
    };

    let result = McGuinnessBuilder::build(input).expect("Build should succeed");

    // Functional inspection via horned-owl iteration
    let declared_classes_count = result
        .ontology
        .iter()
        .filter(|a| matches!(&a.component, Component::DeclareClass(_)))
        .count();

    let subclass_axioms_count = result
        .ontology
        .iter()
        .filter(|a| matches!(&a.component, Component::SubClassOf(_)))
        .count();

    let obj_prop_domain_count = result
        .ontology
        .iter()
        .filter(|a| matches!(&a.component, Component::ObjectPropertyDomain(_)))
        .count();

    assert_eq!(declared_classes_count, 2);
    assert_eq!(subclass_axioms_count, 1);
    assert_eq!(obj_prop_domain_count, 1);
}
