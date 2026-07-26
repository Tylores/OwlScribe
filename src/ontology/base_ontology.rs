use anyhow::{Context, Result};
use horned_owl::io::ParserConfiguration;
use horned_owl::model::*;
use horned_owl::ontology::set::SetOntology;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeedClass {
    pub name: String,
    pub iri: String,
    pub comment: Option<String>,
    pub synonyms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeedProperty {
    pub name: String,
    pub iri: String,
    pub property_type: String, // "object" or "data"
    pub domain: Option<String>,
    pub range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseOntologySeed {
    pub ontology_iri: String,
    pub prefix: Option<String>,
    pub top_classes: Vec<SeedClass>,
    pub key_properties: Vec<SeedProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedMatchResult {
    pub target_iri: String,
    pub concept_name: String,
    pub suggested_mapping: String, // "equivalentClass" or "subClassOf"
    pub confidence_boost: f64,
}

pub struct BaseOntologyLoader;

impl BaseOntologyLoader {
    pub fn from_ofn_str(ofn_content: &str) -> Result<(SetOntology<ArcStr>, BaseOntologySeed)> {
        let mut cursor = std::io::Cursor::new(ofn_content.as_bytes());
        let config = ParserConfiguration::default();
        let (ontology, _prefix_mapping) = horned_owl::io::ofn::reader::read(&mut cursor, config)
            .map_err(|e| anyhow::anyhow!("Failed to parse OWL Functional Syntax: {:?}", e))?;

        let seed = Self::extract_seed_from_ontology(&ontology)?;
        Ok((ontology, seed))
    }

    pub fn from_ofn_file<P: AsRef<Path>>(path: P) -> Result<(SetOntology<ArcStr>, BaseOntologySeed)> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open base ontology file at '{}'", path.as_ref().display()))?;
        let mut reader = BufReader::new(file);
        let config = ParserConfiguration::default();
        let (ontology, _prefix_mapping) = horned_owl::io::ofn::reader::read(&mut reader, config)
            .map_err(|e| anyhow::anyhow!("Failed to parse OWL Functional Syntax file: {:?}", e))?;

        let seed = Self::extract_seed_from_ontology(&ontology)?;
        Ok((ontology, seed))
    }

    fn extract_seed_from_ontology(
        ontology: &SetOntology<ArcStr>,
    ) -> Result<BaseOntologySeed> {
        let mut ontology_iri = "http://example.org/base#".to_string();
        for ann_component in ontology.iter() {
            if let Component::OntologyID(id) = &ann_component.component {
                if let Some(iri) = &id.iri {
                    ontology_iri = iri.as_ref().to_string();
                }
            }
        }

        let mut top_classes = Vec::new();
        let mut key_properties = Vec::new();

        for ann_component in ontology.iter() {
            match &ann_component.component {
                Component::DeclareClass(DeclareClass(class)) => {
                    let iri_str = class.0.as_ref();
                    let name = Self::extract_local_name(iri_str);
                    if !top_classes.iter().any(|c: &SeedClass| c.name == name) {
                        top_classes.push(SeedClass {
                            name: name.clone(),
                            iri: iri_str.to_string(),
                            comment: None,
                            synonyms: vec![],
                        });
                    }
                }
                Component::DeclareObjectProperty(DeclareObjectProperty(op)) => {
                    let iri_str = op.0.as_ref();
                    let name = Self::extract_local_name(iri_str);
                    if !key_properties.iter().any(|p: &SeedProperty| p.name == name) {
                        key_properties.push(SeedProperty {
                            name,
                            iri: iri_str.to_string(),
                            property_type: "object".to_string(),
                            domain: None,
                            range: None,
                        });
                    }
                }
                Component::DeclareDataProperty(DeclareDataProperty(dp)) => {
                    let iri_str = dp.0.as_ref();
                    let name = Self::extract_local_name(iri_str);
                    if !key_properties.iter().any(|p: &SeedProperty| p.name == name) {
                        key_properties.push(SeedProperty {
                            name,
                            iri: iri_str.to_string(),
                            property_type: "data".to_string(),
                            domain: None,
                            range: None,
                        });
                    }
                }
                _ => {}
            }
        }

        let prefix = ontology_iri
            .split('/')
            .filter(|s| !s.is_empty())
            .last()
            .map(|s| s.trim_matches('#').to_string());

        Ok(BaseOntologySeed {
            ontology_iri,
            prefix,
            top_classes,
            key_properties,
        })
    }

    fn extract_local_name(iri: &str) -> String {
        if let Some(pos) = iri.rfind('#') {
            iri[pos + 1..].to_string()
        } else if let Some(pos) = iri.rfind('/') {
            iri[pos + 1..].to_string()
        } else {
            iri.to_string()
        }
    }
}

pub struct SeedConceptMatcher;

impl SeedConceptMatcher {
    pub fn match_term(term: &str, definition: &str, seed: &BaseOntologySeed) -> Option<SeedMatchResult> {
        let norm_term = Self::normalize(term);
        let norm_def = Self::normalize(definition);

        for seed_class in &seed.top_classes {
            let norm_class = Self::normalize(&seed_class.name);

            // Exact match
            if norm_term == norm_class {
                return Some(SeedMatchResult {
                    target_iri: seed_class.iri.clone(),
                    concept_name: seed_class.name.clone(),
                    suggested_mapping: "equivalentClass".to_string(),
                    confidence_boost: 0.25,
                });
            }

            // Synonym match
            if seed_class.synonyms.iter().any(|syn| Self::normalize(syn) == norm_term) {
                return Some(SeedMatchResult {
                    target_iri: seed_class.iri.clone(),
                    concept_name: seed_class.name.clone(),
                    suggested_mapping: "equivalentClass".to_string(),
                    confidence_boost: 0.20,
                });
            }

            // Synonym or substring match (e.g. "Sensing Unit" -> "Sensor" or "Temperature Sensor" -> "Sensor")
            if norm_term.contains(&norm_class) || norm_class.contains(&norm_term) {
                return Some(SeedMatchResult {
                    target_iri: seed_class.iri.clone(),
                    concept_name: seed_class.name.clone(),
                    suggested_mapping: "equivalentClass".to_string(),
                    confidence_boost: 0.15,
                });
            }

            // Definition mentions class concept prominently
            if norm_def.contains(&format!(" {} ", norm_class)) || norm_def.starts_with(&norm_class) {
                return Some(SeedMatchResult {
                    target_iri: seed_class.iri.clone(),
                    concept_name: seed_class.name.clone(),
                    suggested_mapping: "subClassOf".to_string(),
                    confidence_boost: 0.10,
                });
            }
        }

        None
    }


    fn normalize(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ofn_parser_and_seed_extraction() {
        let ofn = r#"Prefix(:=<http://www.w3.org/ns/sosa/>)
Ontology(<http://www.w3.org/ns/sosa/>
Declaration(Class(<http://www.w3.org/ns/sosa/Sensor>))
Declaration(Class(<http://www.w3.org/ns/sosa/Observation>))
Declaration(ObjectProperty(<http://www.w3.org/ns/sosa/madeObservation>))
)"#;
        let (_ont, seed) = BaseOntologyLoader::from_ofn_str(ofn).unwrap();
        assert_eq!(seed.ontology_iri, "http://www.w3.org/ns/sosa/");
        assert_eq!(seed.top_classes.len(), 2);
        assert_eq!(seed.key_properties.len(), 1);
        assert!(seed.top_classes.iter().any(|c| c.name == "Sensor"));
    }

    #[test]
    fn test_seed_concept_matcher() {
        let seed = BaseOntologySeed {
            ontology_iri: "http://www.w3.org/ns/sosa/".to_string(),
            prefix: Some("sosa".to_string()),
            top_classes: vec![SeedClass {
                name: "Sensor".to_string(),
                iri: "http://www.w3.org/ns/sosa/Sensor".to_string(),
                comment: None,
                synonyms: vec!["Sensing Unit".to_string()],
            }],
            key_properties: vec![],
        };

        let match1 = SeedConceptMatcher::match_term("Sensor", "A device that measures a physical property.", &seed);
        assert!(match1.is_some());
        assert_eq!(match1.as_ref().unwrap().suggested_mapping, "equivalentClass");

        let match2 = SeedConceptMatcher::match_term("Sensing Unit", "A hardware component performing sensing.", &seed);
        assert!(match2.is_some());
        assert_eq!(match2.unwrap().concept_name, "Sensor");
    }
}
