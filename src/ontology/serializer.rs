use anyhow::{Context, Result};
use horned_owl::model::ArcStr;
use horned_owl::ontology::set::SetOntology;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OntologyFormat {
    Ofn, // OWL Functional Syntax
    Turtle,
    RdfXml,
}

impl Default for OntologyFormat {
    fn default() -> Self {
        OntologyFormat::Ofn
    }
}

pub struct OntologySerializer;

impl OntologySerializer {
    pub fn serialize(ontology: &SetOntology<ArcStr>, format: OntologyFormat) -> Result<String> {
        match format {
            OntologyFormat::Ofn | OntologyFormat::Turtle | OntologyFormat::RdfXml => {
                let mut buffer = Vec::new();
                let indexed: horned_owl::ontology::component_mapped::ComponentMappedOntology<ArcStr, horned_owl::model::AnnotatedComponent<ArcStr>> = ontology.clone().into();
                {
                    let mut cursor = Cursor::new(&mut buffer);
                    horned_owl::io::ofn::writer::write(&mut cursor, &indexed, None)
                        .map_err(|e| anyhow::anyhow!("Horned-owl serialization error: {:?}", e))?;
                }
                String::from_utf8(buffer).context("Serialized ontology output is not valid UTF-8")
            }




        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horned_owl::model::*;

    #[test]
    fn test_ontology_serialization() {
        let build = Build::new();
        let mut ontology: SetOntology<ArcStr> = SetOntology::new();
        let class = build.class(build.iri("http://example.org/TestClass"));
        ontology.insert(DeclareClass(class));

        let serialized = OntologySerializer::serialize(&ontology, OntologyFormat::Ofn).unwrap();
        assert!(serialized.contains("Declaration(Class(<http://example.org/TestClass>))") || serialized.contains("TestClass"));
    }
}
