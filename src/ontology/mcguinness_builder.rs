use anyhow::Result;
use horned_owl::model::*;
use horned_owl::ontology::set::SetOntology;
use serde::{Deserialize, Serialize};


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





        let mut class_count = 0;
        let mut object_prop_count = 0;
        let mut data_prop_count = 0;
        let mut individual_count = 0;

        // Step 4: Define Classes & Class Hierarchy
        for class_def in &input.classes {
            let class_iri_str = format!("{}{}", base_iri, Self::sanitize_identifier(&class_def.name));
            let class = build.class(build.iri(class_iri_str.as_str()));
            ontology.insert(DeclareClass(class.clone()));
            class_count += 1;

            if let Some(ref parent_name) = class_def.parent_class {
                let parent_iri_str = format!("{}{}", base_iri, Self::sanitize_identifier(parent_name));
                let parent_class = build.class(build.iri(parent_iri_str.as_str()));
                let sub_axiom = SubClassOf {
                    sub: ClassExpression::Class(class.clone()),
                    sup: ClassExpression::Class(parent_class),
                };
                ontology.insert(sub_axiom);
            }
        }

        // Step 5 & 6: Object Properties, Domains, Ranges
        for op_def in &input.object_properties {
            let op_iri_str = format!("{}{}", base_iri, Self::sanitize_identifier(&op_def.name));
            let op = build.object_property(build.iri(op_iri_str.as_str()));
            ontology.insert(DeclareObjectProperty(op.clone()));
            object_prop_count += 1;

            if let Some(ref dom_name) = op_def.domain {
                let dom_iri_str = format!("{}{}", base_iri, Self::sanitize_identifier(dom_name));
                let dom_class = build.class(build.iri(dom_iri_str.as_str()));
                let dom_axiom = ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(op.clone()),
                    ce: ClassExpression::Class(dom_class),
                };
                ontology.insert(dom_axiom);
            }

            if let Some(ref range_name) = op_def.range {
                let range_iri_str = format!("{}{}", base_iri, Self::sanitize_identifier(range_name));
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
            let dp_iri_str = format!("{}{}", base_iri, Self::sanitize_identifier(&dp_def.name));
            let dp = build.data_property(build.iri(dp_iri_str.as_str()));
            ontology.insert(DeclareDataProperty(dp.clone()));
            data_prop_count += 1;

            if let Some(ref dom_name) = dp_def.domain {
                let dom_iri_str = format!("{}{}", base_iri, Self::sanitize_identifier(dom_name));
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

        Ok(McGuinnessOntologyResult {
            ontology,
            ontology_iri: input.ontology_iri,
            class_count,
            object_property_count: object_prop_count,
            data_property_count: data_prop_count,
            individual_count,
            axiom_count,
        })
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
        };

        let result = McGuinnessBuilder::build(input).unwrap();
        assert_eq!(result.class_count, 2);
        assert_eq!(result.object_property_count, 1);
        assert_eq!(result.data_property_count, 1);
        assert_eq!(result.individual_count, 1);
        assert!(result.axiom_count >= 5);
    }
}
