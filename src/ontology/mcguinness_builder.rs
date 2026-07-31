use crate::ontology::base_ontology::BaseOntologyLoader;
use crate::ontology::saref_patterns::{SarefPattern, SarefPatternRegistry, SAREF_CORE_IRI};
use anyhow::Result;
use horned_owl::model::*;
use horned_owl::ontology::set::SetOntology;
use serde::{Deserialize, Serialize};

fn default_mapping_type() -> String {
    "equivalentClass".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDefinition {
    pub name: String,
    pub parent_class: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectPropertyDefinition {
    pub name: String,
    pub domain: Option<String>,
    pub range: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPropertyDefinition {
    pub name: String,
    pub domain: Option<String>,
    pub range: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualDefinition {
    pub name: String,
    pub class_name: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassMapping {
    pub term: String,
    pub target_iri: String,
    #[serde(default = "default_mapping_type")]
    pub mapping_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyMapping {
    pub property_name: String,
    pub target_iri: String,
    #[serde(default = "default_mapping_type")]
    pub mapping_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McGuinnessOntologyInput {
    pub ontology_iri: String,
    pub prefix: Option<String>,
    pub classes: Vec<ClassDefinition>,
    #[serde(default)]
    pub object_properties: Vec<ObjectPropertyDefinition>,
    #[serde(default)]
    pub data_properties: Vec<DataPropertyDefinition>,
    #[serde(default)]
    pub individuals: Vec<IndividualDefinition>,
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub base_ontology_path: Option<String>,
    #[serde(default)]
    pub base_ontology_content: Option<String>,
    #[serde(default)]
    pub class_mappings: Vec<ClassMapping>,
    #[serde(default)]
    pub property_mappings: Vec<PropertyMapping>,
    #[serde(default)]
    pub saref_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaxonomyAuditReport {
    pub total_classes: usize,
    pub subclass_axioms: usize,
    pub object_property_domain_axioms: usize,
    pub object_property_range_axioms: usize,
    pub orphaned_classes: Vec<String>,
}

#[derive(Debug)]
pub struct McGuinnessOntologyResult {
    pub ontology: SetOntology<ArcStr>,
    pub ontology_iri: String,
    pub class_count: usize,
    pub object_property_count: usize,
    pub data_property_count: usize,
    pub individual_count: usize,
    pub axiom_count: usize,
    pub taxonomy_audit: TaxonomyAuditReport,
}

pub struct McGuinnessBuilder;

impl McGuinnessBuilder {
    pub fn build(input: McGuinnessOntologyInput) -> Result<McGuinnessOntologyResult> {
        let build = Build::new();

        let base_iri = if input.ontology_iri.ends_with('#') || input.ontology_iri.ends_with('/') {
            input.ontology_iri.clone()
        } else {
            format!("{}#", input.ontology_iri)
        };

        let ont_iri = build.iri(input.ontology_iri.as_str());
        let mut ontology: SetOntology<ArcStr> = SetOntology::new();
        ontology.insert(Component::OntologyID(OntologyID::new(Some(ont_iri), None)));

        // Base Graph Loading & Merging (Supports .ofn, .rdf, .ttl)
        if let Some(ref path) = input.base_ontology_path {
            let (base_ont, _seed) = BaseOntologyLoader::from_file(path)?;
            for component in base_ont.iter() {
                if !matches!(&component.component, Component::OntologyID(_)) {
                    ontology.insert(component.clone());
                }
            }
        } else if let Some(ref content) = input.base_ontology_content {
            let (base_ont, _seed) = BaseOntologyLoader::from_str(content)?;
            for component in base_ont.iter() {
                if !matches!(&component.component, Component::OntologyID(_)) {
                    ontology.insert(component.clone());
                }
            }
        }

        // SAREF Design Pattern Baselines Injection
        for pat_str in &input.saref_patterns {
            if let Some(pattern) = SarefPattern::from_str(pat_str) {
                SarefPatternRegistry::apply_pattern(&pattern, &mut ontology);
            }
        }

        // OWL Imports
        for import_iri in &input.imports {
            let imp_iri = build.iri(import_iri.as_str());
            ontology.insert(Import(imp_iri));
        }

        let mut class_count = 0;
        let mut object_prop_count = 0;
        let mut data_prop_count = 0;
        let mut individual_count = 0;

        let saref_core_classes = [
            "FeatureOfInterest", "Property", "Measurement", "Function",
            "Command", "Device", "State", "Task", "Commodity", "System", "UnitOfMeasure"
        ];

        // Step 4: Define Classes & Class Hierarchy
        for class_def in &input.classes {
            let is_saref_core = saref_core_classes.iter().any(|&sc| sc.eq_ignore_ascii_case(&class_def.name));
            let class_iri_str = if is_saref_core && !input.saref_patterns.is_empty() {
                format!("{}{}", SAREF_CORE_IRI, Self::sanitize_identifier(&class_def.name))
            } else {
                format!("{}{}", base_iri, Self::sanitize_identifier(&class_def.name))
            };
            let class = build.class(build.iri(class_iri_str.as_str()));
            ontology.insert(DeclareClass(class.clone()));
            class_count += 1;

            if let Some(ref parent_name) = class_def.parent_class {
                let parent_is_saref = saref_core_classes.iter().any(|&sc| sc.eq_ignore_ascii_case(parent_name));
                let parent_iri_str = if parent_name.starts_with("http://") || parent_name.starts_with("https://") {
                    parent_name.clone()
                } else if parent_is_saref && (!input.saref_patterns.is_empty() || parent_name.starts_with("saref:")) {
                    format!("{}{}", SAREF_CORE_IRI, Self::sanitize_identifier(parent_name.trim_start_matches("saref:")))
                } else {
                    format!("{}{}", base_iri, Self::sanitize_identifier(parent_name))
                };
                let parent_class = build.class(build.iri(parent_iri_str.as_str()));
                let sub_axiom = SubClassOf {
                    sub: ClassExpression::Class(class.clone()),
                    sup: ClassExpression::Class(parent_class),
                };
                ontology.insert(sub_axiom);
            }
        }

        // Perform Formal Class Mappings
        for mapping in &input.class_mappings {
            let local_iri_str = if mapping.term.starts_with("http://") || mapping.term.starts_with("https://") {
                mapping.term.clone()
            } else {
                format!("{}{}", base_iri, Self::sanitize_identifier(&mapping.term))
            };
            let target_iri_str = mapping.target_iri.clone();

            let local_class = build.class(build.iri(local_iri_str.as_str()));
            let target_class = build.class(build.iri(target_iri_str.as_str()));

            if mapping.mapping_type == "subClassOf" {
                ontology.insert(SubClassOf {
                    sub: ClassExpression::Class(local_class),
                    sup: ClassExpression::Class(target_class),
                });
            } else {
                // Default to equivalentClass
                ontology.insert(EquivalentClasses(vec![
                    ClassExpression::Class(local_class),
                    ClassExpression::Class(target_class),
                ]));
            }
        }

        // Step 5 & 6: Object Properties, Domains, Ranges
        for op_def in &input.object_properties {
            let is_saref_prop = op_def.name.starts_with("saref:") || ["hasProperty", "isPropertyOf", "makesMeasurement", "relatesToProperty", "isMeasuredIn", "hasFunction", "hasCommand", "actsUpon", "hasSubsystem", "connectsTo", "hasState", "isConsumedBy"].iter().any(|&p| p.eq_ignore_ascii_case(&op_def.name));
            let op_iri_str = if is_saref_prop && !input.saref_patterns.is_empty() {
                format!("{}{}", SAREF_CORE_IRI, Self::sanitize_identifier(op_def.name.trim_start_matches("saref:")))
            } else {
                format!("{}{}", base_iri, Self::sanitize_identifier(&op_def.name))
            };
            let op = build.object_property(build.iri(op_iri_str.as_str()));
            ontology.insert(DeclareObjectProperty(op.clone()));
            object_prop_count += 1;

            if let Some(ref dom_name) = op_def.domain {
                let dom_is_saref = saref_core_classes.iter().any(|&sc| sc.eq_ignore_ascii_case(dom_name));
                let dom_iri_str = if dom_name.starts_with("http://") || dom_name.starts_with("https://") {
                    dom_name.clone()
                } else if (!input.saref_patterns.is_empty() || dom_name.starts_with("saref:")) && dom_is_saref {
                    format!("{}{}", SAREF_CORE_IRI, Self::sanitize_identifier(dom_name.trim_start_matches("saref:")))
                } else {
                    format!("{}{}", base_iri, Self::sanitize_identifier(dom_name))
                };
                let dom_class = build.class(build.iri(dom_iri_str.as_str()));
                let dom_axiom = ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(op.clone()),
                    ce: ClassExpression::Class(dom_class),
                };
                ontology.insert(dom_axiom);
            }

            if let Some(ref range_name) = op_def.range {
                let range_is_saref = saref_core_classes.iter().any(|&sc| sc.eq_ignore_ascii_case(range_name));
                let range_iri_str = if range_name.starts_with("http://") || range_name.starts_with("https://") {
                    range_name.clone()
                } else if (!input.saref_patterns.is_empty() || range_name.starts_with("saref:")) && range_is_saref {
                    format!("{}{}", SAREF_CORE_IRI, Self::sanitize_identifier(range_name.trim_start_matches("saref:")))
                } else {
                    format!("{}{}", base_iri, Self::sanitize_identifier(range_name))
                };
                let range_class = build.class(build.iri(range_iri_str.as_str()));
                let range_axiom = ObjectPropertyRange {
                    ope: ObjectPropertyExpression::ObjectProperty(op.clone()),
                    ce: ClassExpression::Class(range_class),
                };
                ontology.insert(range_axiom);
            }
        }

        // Step 5 & 6: Data Properties, Domains, Ranges
        for dp_def in &input.data_properties {
            let is_saref_dp = dp_def.name.starts_with("saref:") || ["hasValue", "hasTimestamp"].iter().any(|&p| p.eq_ignore_ascii_case(&dp_def.name));
            let dp_iri_str = if is_saref_dp && !input.saref_patterns.is_empty() {
                format!("{}{}", SAREF_CORE_IRI, Self::sanitize_identifier(dp_def.name.trim_start_matches("saref:")))
            } else {
                format!("{}{}", base_iri, Self::sanitize_identifier(&dp_def.name))
            };
            let dp = build.data_property(build.iri(dp_iri_str.as_str()));
            ontology.insert(DeclareDataProperty(dp.clone()));
            data_prop_count += 1;

            if let Some(ref dom_name) = dp_def.domain {
                let dom_is_saref = saref_core_classes.iter().any(|&sc| sc.eq_ignore_ascii_case(dom_name));
                let dom_iri_str = if dom_name.starts_with("http://") || dom_name.starts_with("https://") {
                    dom_name.clone()
                } else if (!input.saref_patterns.is_empty() || dom_name.starts_with("saref:")) && dom_is_saref {
                    format!("{}{}", SAREF_CORE_IRI, Self::sanitize_identifier(dom_name.trim_start_matches("saref:")))
                } else {
                    format!("{}{}", base_iri, Self::sanitize_identifier(dom_name))
                };
                let dom_class = build.class(build.iri(dom_iri_str.as_str()));
                let dom_axiom = DataPropertyDomain {
                    dp: dp.clone(),
                    ce: ClassExpression::Class(dom_class),
                };
                ontology.insert(dom_axiom);
            }

            if let Some(ref range_type) = dp_def.range {
                let range_iri_str = if range_type.starts_with("http") {
                    range_type.clone()
                } else {
                    format!("http://www.w3.org/2001/XMLSchema#{}", range_type.trim_start_matches("xsd:"))
                };
                let dt = build.datatype(build.iri(range_iri_str.as_str()));
                let range_axiom = DataPropertyRange {
                    dp: dp.clone(),
                    dr: DataRange::Datatype(dt),
                };
                ontology.insert(range_axiom);
            }
        }

        // Step 7: Create Instances / Individuals
        for ind_def in &input.individuals {
            let ind_iri_str = format!("{}{}", base_iri, Self::sanitize_identifier(&ind_def.name));
            let ind = build.named_individual(build.iri(ind_iri_str.as_str()));
            ontology.insert(DeclareNamedIndividual(ind.clone()));
            individual_count += 1;

            let class_iri_str = format!("{}{}", base_iri, Self::sanitize_identifier(&ind_def.class_name));
            let class = build.class(build.iri(class_iri_str.as_str()));
            let class_assertion = ClassAssertion {
                ce: ClassExpression::Class(class),
                i: Individual::Named(ind),
            };
            ontology.insert(class_assertion);
        }

        let axiom_count = ontology.iter().count();
        let taxonomy_audit = Self::audit_taxonomy_axioms(&ontology);

        Ok(McGuinnessOntologyResult {
            ontology,
            ontology_iri: input.ontology_iri,
            class_count,
            object_property_count: object_prop_count,
            data_property_count: data_prop_count,
            individual_count,
            axiom_count,
            taxonomy_audit,
        })
    }

    pub fn audit_taxonomy_axioms(ontology: &SetOntology<ArcStr>) -> TaxonomyAuditReport {
        let mut declared_classes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut sub_classes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut super_classes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut subclass_axioms = 0;
        let mut op_domain_axioms = 0;
        let mut op_range_axioms = 0;

        for component in ontology.iter() {
            match &component.component {
                Component::DeclareClass(DeclareClass(c)) => {
                    declared_classes.insert(c.0.as_ref().to_string());
                }
                Component::SubClassOf(SubClassOf { sub, sup }) => {
                    subclass_axioms += 1;
                    if let ClassExpression::Class(c) = sub {
                        sub_classes.insert(c.0.as_ref().to_string());
                    }
                    if let ClassExpression::Class(c) = sup {
                        super_classes.insert(c.0.as_ref().to_string());
                    }
                }
                Component::ObjectPropertyDomain(_) => {
                    op_domain_axioms += 1;
                }
                Component::ObjectPropertyRange(_) => {
                    op_range_axioms += 1;
                }
                _ => {}
            }
        }

        let total_classes = declared_classes.len();

        let orphaned_classes: Vec<String> = declared_classes
            .into_iter()
            .filter(|cls| !sub_classes.contains(cls) && !super_classes.contains(cls))
            .collect();

        TaxonomyAuditReport {
            total_classes,
            subclass_axioms,
            object_property_domain_axioms: op_domain_axioms,
            object_property_range_axioms: op_range_axioms,
            orphaned_classes,
        }
    }

    fn sanitize_identifier(raw: &str) -> String {
        let mut clean = raw
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        if clean.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            clean = format!("_{}", clean);
        }
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcguinness_builder_basic() {
        let input = McGuinnessOntologyInput {
            ontology_iri: "http://example.org/test#".to_string(),
            prefix: Some("ex".to_string()),
            classes: vec![
                ClassDefinition {
                    name: "Document".to_string(),
                    parent_class: None,
                    comment: Some("Base Document Class".to_string()),
                },
                ClassDefinition {
                    name: "Specification".to_string(),
                    parent_class: Some("Document".to_string()),
                    comment: None,
                },
            ],
            object_properties: vec![ObjectPropertyDefinition {
                name: "references".to_string(),
                domain: Some("Specification".to_string()),
                range: Some("Document".to_string()),
                comment: None,
            }],
            data_properties: vec![DataPropertyDefinition {
                name: "hasTitle".to_string(),
                domain: Some("Document".to_string()),
                range: Some("xsd:string".to_string()),
                comment: None,
            }],
            individuals: vec![IndividualDefinition {
                name: "ISO12345_Instance".to_string(),
                class_name: "Specification".to_string(),
                comment: None,
            }],
            imports: vec!["http://www.w3.org/ns/sosa/".to_string()],
            base_ontology_path: None,
            base_ontology_content: None,
            class_mappings: vec![ClassMapping {
                term: "Specification".to_string(),
                target_iri: "http://www.w3.org/ns/sosa/Observation".to_string(),
                mapping_type: "equivalentClass".to_string(),
            }],
            property_mappings: vec![],
            saref_patterns: vec![],
        };

        let result = McGuinnessBuilder::build(input).unwrap();
        assert_eq!(result.class_count, 2);
        assert_eq!(result.object_property_count, 1);
        assert_eq!(result.data_property_count, 1);
        assert_eq!(result.individual_count, 1);
        assert!(result.axiom_count >= 6);
    }

    #[test]
    fn test_mcguinness_builder_saref_patterns() {
        let input = McGuinnessOntologyInput {
            ontology_iri: "http://example.org/grid#".to_string(),
            prefix: Some("grid".to_string()),
            classes: vec![ClassDefinition {
                name: "SmartMeter".to_string(),
                parent_class: Some("Device".to_string()),
                comment: Some("Smart grid meter device".to_string()),
            }],
            object_properties: vec![],
            data_properties: vec![],
            individuals: vec![],
            imports: vec![],
            base_ontology_path: None,
            base_ontology_content: None,
            class_mappings: vec![],
            property_mappings: vec![],
            saref_patterns: vec!["feature_of_interest".to_string(), "measurement".to_string()],
        };

        let result = McGuinnessBuilder::build(input).unwrap();
        assert!(result.axiom_count >= 10);
    }
}
