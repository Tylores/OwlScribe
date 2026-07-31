use anyhow::{Context, Result};
use horned_owl::model::ArcStr;
use horned_owl::ontology::set::SetOntology;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

use crate::ontology::mermaid::{MermaidConfig, MermaidTranslator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OntologyFormat {
    Turtle,
    #[serde(alias = "json-ld")]
    JsonLd,
    Ofn, // OWL Functional Syntax
    RdfXml,
    Mermaid,
}

impl Default for OntologyFormat {
    fn default() -> Self {
        OntologyFormat::Turtle
    }
}

pub struct OntologySerializer;

impl OntologySerializer {
    pub fn serialize(ontology: &SetOntology<ArcStr>, format: OntologyFormat) -> Result<String> {
        match format {
            OntologyFormat::Turtle => Self::serialize_turtle(ontology),
            OntologyFormat::JsonLd => Self::serialize_jsonld(ontology),
            OntologyFormat::Ofn => {
                let mut buffer = Vec::new();
                let indexed: horned_owl::ontology::component_mapped::ComponentMappedOntology<ArcStr, horned_owl::model::AnnotatedComponent<ArcStr>> = ontology.clone().into();
                {
                    let mut cursor = Cursor::new(&mut buffer);
                    horned_owl::io::ofn::writer::write(&mut cursor, &indexed, None)
                        .map_err(|e| anyhow::anyhow!("Horned-owl serialization error: {:?}", e))?;
                }
                String::from_utf8(buffer).context("Serialized ontology output is not valid UTF-8")
            }
            OntologyFormat::RdfXml => Self::serialize_turtle(ontology),
            OntologyFormat::Mermaid => MermaidTranslator::translate(ontology, &MermaidConfig::default()),
        }
    }


    fn serialize_jsonld(ontology: &SetOntology<ArcStr>) -> Result<String> {
        use horned_owl::model::*;
        use serde_json::{json, Map, Value};

        let mut graph: Vec<Value> = Vec::new();
        let mut ontology_obj: Map<String, Value> = Map::new();
        let mut imports_list: Vec<Value> = Vec::new();

        let mut node_map: std::collections::BTreeMap<String, Map<String, Value>> = std::collections::BTreeMap::new();

        for component in ontology.iter() {
            match &component.component {
                Component::OntologyID(id) => {
                    if let Some(ref iri) = id.iri {
                        ontology_obj.insert("@id".to_string(), json!(iri.as_ref()));
                        ontology_obj.insert("@type".to_string(), json!("owl:Ontology"));
                    }
                }
                Component::Import(Import(iri)) => {
                    imports_list.push(json!({ "@id": iri.as_ref() }));
                }
                Component::DeclareClass(DeclareClass(c)) => {
                    let iri = c.0.as_ref().to_string();
                    let entry = node_map.entry(iri.clone()).or_insert_with(Map::new);
                    entry.insert("@id".to_string(), json!(iri));
                    entry.insert("@type".to_string(), json!("owl:Class"));
                }
                Component::DeclareObjectProperty(DeclareObjectProperty(op)) => {
                    let iri = op.0.as_ref().to_string();
                    let entry = node_map.entry(iri.clone()).or_insert_with(Map::new);
                    entry.insert("@id".to_string(), json!(iri));
                    entry.insert("@type".to_string(), json!("owl:ObjectProperty"));
                }
                Component::DeclareDataProperty(DeclareDataProperty(dp)) => {
                    let iri = dp.0.as_ref().to_string();
                    let entry = node_map.entry(iri.clone()).or_insert_with(Map::new);
                    entry.insert("@id".to_string(), json!(iri));
                    entry.insert("@type".to_string(), json!("owl:DataProperty"));
                }
                Component::DeclareNamedIndividual(DeclareNamedIndividual(ind)) => {
                    let iri = ind.0.as_ref().to_string();
                    let entry = node_map.entry(iri.clone()).or_insert_with(Map::new);
                    entry.insert("@id".to_string(), json!(iri));
                    entry.insert("@type".to_string(), json!("owl:NamedIndividual"));
                }
                Component::SubClassOf(SubClassOf { sub, sup }) => {
                    if let (ClassExpression::Class(sub_c), ClassExpression::Class(sup_c)) = (sub, sup) {
                        let sub_iri = sub_c.0.as_ref().to_string();
                        let sup_iri = sup_c.0.as_ref().to_string();
                        let entry = node_map.entry(sub_iri.clone()).or_insert_with(Map::new);
                        entry.insert("@id".to_string(), json!(sub_iri));
                        entry.insert("rdfs:subClassOf".to_string(), json!({ "@id": sup_iri }));
                    }
                }
                Component::EquivalentClasses(EquivalentClasses(exprs)) => {
                    let class_iris: Vec<String> = exprs.iter().filter_map(|e| {
                        if let ClassExpression::Class(c) = e {
                            Some(c.0.as_ref().to_string())
                        } else {
                            None
                        }
                    }).collect();
                    if class_iris.len() >= 2 {
                        let c1 = &class_iris[0];
                        let c2 = &class_iris[1];
                        let entry = node_map.entry(c1.clone()).or_insert_with(Map::new);
                        entry.insert("@id".to_string(), json!(c1));
                        entry.insert("owl:equivalentClass".to_string(), json!({ "@id": c2 }));
                    }
                }
                Component::ObjectPropertyDomain(ObjectPropertyDomain { ope, ce }) => {
                    if let (ObjectPropertyExpression::ObjectProperty(op), ClassExpression::Class(c)) = (ope, ce) {
                        let op_iri = op.0.as_ref().to_string();
                        let c_iri = c.0.as_ref().to_string();
                        let entry = node_map.entry(op_iri.clone()).or_insert_with(Map::new);
                        entry.insert("@id".to_string(), json!(op_iri));
                        entry.insert("rdfs:domain".to_string(), json!({ "@id": c_iri }));
                    }
                }
                Component::ObjectPropertyRange(ObjectPropertyRange { ope, ce }) => {
                    if let (ObjectPropertyExpression::ObjectProperty(op), ClassExpression::Class(c)) = (ope, ce) {
                        let op_iri = op.0.as_ref().to_string();
                        let c_iri = c.0.as_ref().to_string();
                        let entry = node_map.entry(op_iri.clone()).or_insert_with(Map::new);
                        entry.insert("@id".to_string(), json!(op_iri));
                        entry.insert("rdfs:range".to_string(), json!({ "@id": c_iri }));
                    }
                }
                Component::DataPropertyDomain(DataPropertyDomain { dp, ce }) => {
                    if let ClassExpression::Class(c) = ce {
                        let dp_iri = dp.0.as_ref().to_string();
                        let c_iri = c.0.as_ref().to_string();
                        let entry = node_map.entry(dp_iri.clone()).or_insert_with(Map::new);
                        entry.insert("@id".to_string(), json!(dp_iri));
                        entry.insert("rdfs:domain".to_string(), json!({ "@id": c_iri }));
                    }
                }
                Component::DataPropertyRange(DataPropertyRange { dp, dr }) => {
                    if let DataRange::Datatype(dt) = dr {
                        let dp_iri = dp.0.as_ref().to_string();
                        let dt_iri = dt.0.as_ref().to_string();
                        let entry = node_map.entry(dp_iri.clone()).or_insert_with(Map::new);
                        entry.insert("@id".to_string(), json!(dp_iri));
                        entry.insert("rdfs:range".to_string(), json!({ "@id": dt_iri }));
                    }
                }
                Component::ClassAssertion(ClassAssertion { ce, i }) => {
                    if let (ClassExpression::Class(c), Individual::Named(ind)) = (ce, i) {
                        let ind_iri = ind.0.as_ref().to_string();
                        let c_iri = c.0.as_ref().to_string();
                        let entry = node_map.entry(ind_iri.clone()).or_insert_with(Map::new);
                        entry.insert("@id".to_string(), json!(ind_iri));
                        entry.insert("@type".to_string(), json!(c_iri));
                    }
                }
                _ => {}
            }
        }

        if !imports_list.is_empty() {
            ontology_obj.insert("owl:imports".to_string(), Value::Array(imports_list));
        }

        if !ontology_obj.is_empty() {
            graph.push(Value::Object(ontology_obj));
        }

        for (_iri, node) in node_map {
            graph.push(Value::Object(node));
        }

        let root = json!({
            "@context": {
                "owl": "http://www.w3.org/2002/07/owl#",
                "rdf": "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
                "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
                "xsd": "http://www.w3.org/2001/XMLSchema#"
            },
            "@graph": graph
        });

        serde_json::to_string_pretty(&root).context("Failed to serialize JSON-LD ontology")
    }

    fn serialize_turtle(ontology: &SetOntology<ArcStr>) -> Result<String> {
        use horned_owl::model::*;
        let mut out = String::new();
        out.push_str("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n");
        out.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
        out.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
        out.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
        out.push_str("@prefix saref: <https://saref.etsi.org/core/> .\n\n");

        let mut ontology_iri = None;
        let mut imports = Vec::new();
        let mut classes = Vec::new();
        let mut obj_props = Vec::new();
        let mut data_props = Vec::new();
        let mut individuals = Vec::new();
        let mut sub_classes = Vec::new();
        let mut equiv_classes = Vec::new();
        let mut obj_domains = Vec::new();
        let mut obj_ranges = Vec::new();
        let mut data_domains = Vec::new();
        let mut data_ranges = Vec::new();
        let mut class_assertions = Vec::new();

        for component in ontology.iter() {
            match &component.component {
                Component::OntologyID(id) => {
                    if let Some(ref iri) = id.iri {
                        ontology_iri = Some(iri.as_ref().to_string());
                    }
                }
                Component::Import(Import(iri)) => {
                    imports.push(iri.as_ref().to_string());
                }
                Component::DeclareClass(DeclareClass(c)) => {
                    classes.push(c.0.as_ref().to_string());
                }
                Component::DeclareObjectProperty(DeclareObjectProperty(op)) => {
                    obj_props.push(op.0.as_ref().to_string());
                }
                Component::DeclareDataProperty(DeclareDataProperty(dp)) => {
                    data_props.push(dp.0.as_ref().to_string());
                }
                Component::DeclareNamedIndividual(DeclareNamedIndividual(ind)) => {
                    individuals.push(ind.0.as_ref().to_string());
                }
                Component::SubClassOf(SubClassOf { sub, sup }) => {
                    if let (ClassExpression::Class(sub_c), ClassExpression::Class(sup_c)) = (sub, sup) {
                        sub_classes.push((sub_c.0.as_ref().to_string(), sup_c.0.as_ref().to_string()));
                    }
                }
                Component::EquivalentClasses(EquivalentClasses(exprs)) => {
                    let class_iris: Vec<String> = exprs.iter().filter_map(|e| {
                        if let ClassExpression::Class(c) = e {
                            Some(c.0.as_ref().to_string())
                        } else {
                            None
                        }
                    }).collect();
                    if class_iris.len() >= 2 {
                        equiv_classes.push((class_iris[0].clone(), class_iris[1].clone()));
                    }
                }
                Component::ObjectPropertyDomain(ObjectPropertyDomain { ope, ce }) => {
                    if let (ObjectPropertyExpression::ObjectProperty(op), ClassExpression::Class(c)) = (ope, ce) {
                        obj_domains.push((op.0.as_ref().to_string(), c.0.as_ref().to_string()));
                    }
                }
                Component::ObjectPropertyRange(ObjectPropertyRange { ope, ce }) => {
                    if let (ObjectPropertyExpression::ObjectProperty(op), ClassExpression::Class(c)) = (ope, ce) {
                        obj_ranges.push((op.0.as_ref().to_string(), c.0.as_ref().to_string()));
                    }
                }
                Component::DataPropertyDomain(DataPropertyDomain { dp, ce }) => {
                    if let ClassExpression::Class(c) = ce {
                        data_domains.push((dp.0.as_ref().to_string(), c.0.as_ref().to_string()));
                    }
                }
                Component::DataPropertyRange(DataPropertyRange { dp, dr }) => {
                    if let DataRange::Datatype(dt) = dr {
                        data_ranges.push((dp.0.as_ref().to_string(), dt.0.as_ref().to_string()));
                    }
                }
                Component::ClassAssertion(ClassAssertion { ce, i }) => {
                    if let (ClassExpression::Class(c), Individual::Named(ind)) = (ce, i) {
                        class_assertions.push((ind.0.as_ref().to_string(), c.0.as_ref().to_string()));
                    }
                }
                _ => {}
            }
        }

        if let Some(ont_iri) = ontology_iri {
            out.push_str(&format!("<{}> a owl:Ontology", ont_iri));
            for imp in &imports {
                out.push_str(&format!(" ;\n    owl:imports <{}>", imp));
            }
            out.push_str(" .\n\n");
        }

        for c in &classes {
            out.push_str(&format!("<{}> a owl:Class .\n", c));
        }
        for op in &obj_props {
            out.push_str(&format!("<{}> a owl:ObjectProperty .\n", op));
        }
        for dp in &data_props {
            out.push_str(&format!("<{}> a owl:DataProperty .\n", dp));
        }
        for ind in &individuals {
            out.push_str(&format!("<{}> a owl:NamedIndividual .\n", ind));
        }
        if !classes.is_empty() || !obj_props.is_empty() || !data_props.is_empty() || !individuals.is_empty() {
            out.push('\n');
        }

        for (sub, sup) in &sub_classes {
            out.push_str(&format!("<{}> rdfs:subClassOf <{}> .\n", sub, sup));
        }
        for (c1, c2) in &equiv_classes {
            out.push_str(&format!("<{}> owl:equivalentClass <{}> .\n", c1, c2));
        }
        for (op, dom) in &obj_domains {
            out.push_str(&format!("<{}> rdfs:domain <{}> .\n", op, dom));
        }
        for (op, ran) in &obj_ranges {
            out.push_str(&format!("<{}> rdfs:range <{}> .\n", op, ran));
        }
        for (dp, dom) in &data_domains {
            out.push_str(&format!("<{}> rdfs:domain <{}> .\n", dp, dom));
        }
        for (dp, ran) in &data_ranges {
            out.push_str(&format!("<{}> rdfs:range <{}> .\n", dp, ran));
        }
        for (ind, c) in &class_assertions {
            out.push_str(&format!("<{}> a <{}> .\n", ind, c));
        }

        Ok(out)
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

    #[test]
    fn test_turtle_default_serialization() {
        let build = Build::new();
        let mut ontology: SetOntology<ArcStr> = SetOntology::new();
        let class = build.class(build.iri("http://example.org/TestClass"));
        ontology.insert(DeclareClass(class));

        let serialized = OntologySerializer::serialize(&ontology, OntologyFormat::default()).unwrap();
        assert!(serialized.contains("<http://example.org/TestClass> a owl:Class ."));
    }

    #[test]
    fn test_jsonld_ontology_serialization() {
        let build = Build::new();
        let mut ontology: SetOntology<ArcStr> = SetOntology::new();
        let class = build.class(build.iri("http://example.org/TestClass"));
        ontology.insert(DeclareClass(class));

        let serialized = OntologySerializer::serialize(&ontology, OntologyFormat::JsonLd).unwrap();
        assert!(serialized.contains("@context"));
        assert!(serialized.contains("http://example.org/TestClass"));
        assert!(serialized.contains("owl:Class"));
    }
}
