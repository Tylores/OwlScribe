use anyhow::Result;
use horned_owl::model::*;
use horned_owl::ontology::set::SetOntology;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MermaidDiagramType {
    ClassDiagram,
    ErDiagram,
}

impl Default for MermaidDiagramType {
    fn default() -> Self {
        MermaidDiagramType::ClassDiagram
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MermaidConfig {
    #[serde(default)]
    pub diagram_type: MermaidDiagramType,
    #[serde(default)]
    pub focus_classes: Vec<String>,
    #[serde(default)]
    pub max_depth: Option<usize>,
    #[serde(default)]
    pub namespaces: Vec<String>,
    #[serde(default = "default_true")]
    pub include_data_properties: bool,
    #[serde(default = "default_true")]
    pub include_stereotypes: bool,
    #[serde(default = "default_true")]
    pub group_by_namespace: bool,
    #[serde(default)]
    pub max_nodes: Option<usize>,
}

fn default_true() -> bool {
    true
}

impl Default for MermaidConfig {
    fn default() -> Self {
        Self {
            diagram_type: MermaidDiagramType::ClassDiagram,
            focus_classes: Vec::new(),
            max_depth: None,
            namespaces: Vec::new(),
            include_data_properties: true,
            include_stereotypes: true,
            group_by_namespace: true,
            max_nodes: None,
        }
    }
}

pub struct MermaidTranslator;

impl MermaidTranslator {
    pub fn translate(ontology: &SetOntology<ArcStr>, config: &MermaidConfig) -> Result<String> {
        let prefixes = Self::build_prefix_map(ontology);

        let mut classes: BTreeSet<String> = BTreeSet::new();
        let mut object_properties: BTreeMap<String, (Option<String>, Option<String>)> = BTreeMap::new();
        let mut data_properties: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new(); // class_iri -> vec[(prop_name, datatype)]
        let mut sub_class_relations: BTreeSet<(String, String)> = BTreeSet::new(); // (sub, sup)
        let mut equivalent_class_relations: BTreeSet<(String, String)> = BTreeSet::new(); // (c1, c2)

        // Parse horned-owl components
        for component in ontology.iter() {
            match &component.component {
                Component::DeclareClass(DeclareClass(c)) => {
                    let iri = c.0.as_ref().to_string();
                    classes.insert(iri);
                }
                Component::SubClassOf(SubClassOf { sub, sup }) => {
                    if let (ClassExpression::Class(sub_c), ClassExpression::Class(sup_c)) = (sub, sup) {
                        let sub_iri = sub_c.0.as_ref().to_string();
                        let sup_iri = sup_c.0.as_ref().to_string();
                        classes.insert(sub_iri.clone());
                        classes.insert(sup_iri.clone());
                        sub_class_relations.insert((sub_iri, sup_iri));
                    }
                }
                Component::EquivalentClasses(EquivalentClasses(exprs)) => {
                    let mut class_iris = Vec::new();
                    for expr in exprs {
                        if let ClassExpression::Class(c) = expr {
                            let iri = c.0.as_ref().to_string();
                            classes.insert(iri.clone());
                            class_iris.push(iri);
                        }
                    }
                    if class_iris.len() >= 2 {
                        for i in 0..class_iris.len() - 1 {
                            equivalent_class_relations.insert((class_iris[i].clone(), class_iris[i + 1].clone()));
                        }
                    }
                }
                Component::ObjectPropertyDomain(ObjectPropertyDomain { ope, ce }) => {
                    if let (ObjectPropertyExpression::ObjectProperty(op), ClassExpression::Class(c)) = (ope, ce) {
                        let op_iri = op.0.as_ref().to_string();
                        let c_iri = c.0.as_ref().to_string();
                        classes.insert(c_iri.clone());
                        let entry = object_properties.entry(op_iri).or_insert((None, None));
                        entry.0 = Some(c_iri);
                    }
                }
                Component::ObjectPropertyRange(ObjectPropertyRange { ope, ce }) => {
                    if let (ObjectPropertyExpression::ObjectProperty(op), ClassExpression::Class(c)) = (ope, ce) {
                        let op_iri = op.0.as_ref().to_string();
                        let c_iri = c.0.as_ref().to_string();
                        classes.insert(c_iri.clone());
                        let entry = object_properties.entry(op_iri).or_insert((None, None));
                        entry.1 = Some(c_iri);
                    }
                }
                Component::DataPropertyDomain(DataPropertyDomain { dp, ce }) => {
                    if config.include_data_properties {
                        if let ClassExpression::Class(c) = ce {
                            let dp_iri = dp.0.as_ref().to_string();
                            let c_iri = c.0.as_ref().to_string();
                            classes.insert(c_iri.clone());
                            let prop_name = Self::compact_iri(&dp_iri, &prefixes);
                            data_properties.entry(c_iri).or_default().push((prop_name, "xs:string".to_string()));
                        }
                    }
                }
                Component::DataPropertyRange(DataPropertyRange { dp, dr }) => {
                    if config.include_data_properties {
                        let dp_iri = dp.0.as_ref().to_string();
                        let dt_name = match dr {
                            DataRange::Datatype(dt) => Self::compact_iri(dt.0.as_ref(), &prefixes),
                            _ => "xs:string".to_string(),
                        };
                        // Update range if property entry exists
                        for (_cls, props) in data_properties.iter_mut() {
                            for (p_name, p_dt) in props.iter_mut() {
                                if p_name == &Self::compact_iri(&dp_iri, &prefixes) {
                                    *p_dt = dt_name.clone();
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Apply filtering (focus_classes, max_depth, namespaces, max_nodes)
        let filtered_classes = Self::filter_classes(
            &classes,
            &sub_class_relations,
            &equivalent_class_relations,
            &object_properties,
            config,
            &prefixes,
        );

        // Filter relations to only include filtered classes
        let active_subclass: BTreeSet<(String, String)> = sub_class_relations
            .into_iter()
            .filter(|(sub, sup)| filtered_classes.contains(sub) && filtered_classes.contains(sup))
            .collect();

        let active_equiv: BTreeSet<(String, String)> = equivalent_class_relations
            .into_iter()
            .filter(|(c1, c2)| filtered_classes.contains(c1) && filtered_classes.contains(c2))
            .collect();

        let active_obj_props: BTreeMap<String, (String, String)> = object_properties
            .into_iter()
            .filter_map(|(op, (dom, rng))| {
                if let (Some(d), Some(r)) = (dom, rng) {
                    if filtered_classes.contains(&d) && filtered_classes.contains(&r) {
                        return Some((op, (d, r)));
                    }
                }
                None
            })
            .collect();

        match config.diagram_type {
            MermaidDiagramType::ClassDiagram => Self::render_class_diagram(
                &filtered_classes,
                &active_subclass,
                &active_equiv,
                &active_obj_props,
                &data_properties,
                config,
                &prefixes,
            ),
            MermaidDiagramType::ErDiagram => Self::render_er_diagram(
                &filtered_classes,
                &active_subclass,
                &active_equiv,
                &active_obj_props,
                &data_properties,
                config,
                &prefixes,
            ),
        }
    }

    fn build_prefix_map(ontology: &SetOntology<ArcStr>) -> BTreeMap<String, String> {
        let mut prefixes = BTreeMap::new();
        prefixes.insert("http://www.w3.org/2002/07/owl#".to_string(), "owl".to_string());
        prefixes.insert("http://www.w3.org/2000/01/rdf-schema#".to_string(), "rdfs".to_string());
        prefixes.insert("http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(), "rdf".to_string());
        prefixes.insert("http://www.w3.org/2001/XMLSchema#".to_string(), "xsd".to_string());
        prefixes.insert("http://www.w3.org/ns/sosa/".to_string(), "sosa".to_string());
        prefixes.insert("http://www.w3.org/ns/ssn/".to_string(), "ssn".to_string());
        prefixes.insert("https://saref.etsi.org/core/".to_string(), "saref".to_string());
        prefixes.insert("https://saref.etsi.org/saref4grid/".to_string(), "saref4grid".to_string());
        prefixes.insert("https://smartgrid.ieee.org/2030-5/2023#".to_string(), "sep".to_string());
        prefixes.insert("http://example.org/ontologies/common-grid-services#".to_string(), "cgs".to_string());
        prefixes.insert("http://egot.org/ontologies/2024/esi#".to_string(), "esi".to_string());
        prefixes.insert("http://egot.org/ontologies/2024/grid-service-mapping#".to_string(), "gsm".to_string());

        // Extract base ontology IRI if available
        for component in ontology.iter() {
            if let Component::OntologyID(id) = &component.component {
                if let Some(ref iri) = id.iri {
                    let iri_str = iri.as_ref().to_string();
                    if !iri_str.is_empty() && !prefixes.contains_key(&iri_str) {
                        let base_uri = if iri_str.ends_with('#') || iri_str.ends_with('/') {
                            iri_str.clone()
                        } else {
                            format!("{}/", iri_str)
                        };
                        prefixes.insert(base_uri, "ont".to_string());
                    }
                }
            }
        }

        prefixes
    }

    fn compact_iri(iri: &str, prefixes: &BTreeMap<String, String>) -> String {
        for (base, prefix) in prefixes {
            if iri.starts_with(base) {
                let local = &iri[base.len()..];
                if !local.is_empty() {
                    return format!("{}:{}", prefix, local);
                }
            }
        }

        if let Some(idx) = iri.rfind('#') {
            return format!("{}:{}", &iri[..idx], &iri[idx + 1..]);
        }
        if let Some(idx) = iri.rfind('/') {
            return format!("{}:{}", &iri[..idx], &iri[idx + 1..]);
        }

        iri.to_string()
    }

    fn sanitize_id(compact: &str) -> String {
        compact
            .replace(':', "_")
            .replace('/', "_")
            .replace('-', "_")
            .replace('.', "_")
            .replace('#', "_")
    }

    fn filter_classes(
        all_classes: &BTreeSet<String>,
        subclasses: &BTreeSet<(String, String)>,
        equivs: &BTreeSet<(String, String)>,
        obj_props: &BTreeMap<String, (Option<String>, Option<String>)>,
        config: &MermaidConfig,
        prefixes: &BTreeMap<String, String>,
    ) -> BTreeSet<String> {
        let mut result = BTreeSet::new();

        // 1. Initial selection
        if config.focus_classes.is_empty() {
            for c in all_classes {
                result.insert(c.clone());
            }
        } else {
            for c in all_classes {
                let compact = Self::compact_iri(c, prefixes);
                for focus in &config.focus_classes {
                    if c.contains(focus) || compact.contains(focus) {
                        result.insert(c.clone());
                    }
                }
            }
        }

        // Filter by namespaces if provided
        if !config.namespaces.is_empty() {
            result.retain(|c| {
                let compact = Self::compact_iri(c, prefixes);
                config.namespaces.iter().any(|ns| compact.starts_with(ns) || c.contains(ns))
            });
        }

        // 2. Traversal up to max_depth if focus_classes were provided
        if !config.focus_classes.is_empty() && config.max_depth.unwrap_or(0) > 0 {
            let max_depth = config.max_depth.unwrap_or(1);
            let mut visited = result.clone();
            let mut queue: VecDeque<(String, usize)> = result.iter().map(|c| (c.clone(), 0)).collect();

            // Build adjacency list
            let mut adj: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for (sub, sup) in subclasses {
                adj.entry(sub.clone()).or_default().push(sup.clone());
                adj.entry(sup.clone()).or_default().push(sub.clone());
            }
            for (c1, c2) in equivs {
                adj.entry(c1.clone()).or_default().push(c2.clone());
                adj.entry(c2.clone()).or_default().push(c1.clone());
            }
            for (_op, (dom, rng)) in obj_props {
                if let (Some(d), Some(r)) = (dom, rng) {
                    adj.entry(d.clone()).or_default().push(r.clone());
                    adj.entry(r.clone()).or_default().push(d.clone());
                }
            }

            while let Some((curr, depth)) = queue.pop_front() {
                if depth >= max_depth {
                    continue;
                }
                if let Some(neighbors) = adj.get(&curr) {
                    for neighbor in neighbors {
                        if visited.insert(neighbor.clone()) {
                            queue.push_back((neighbor.clone(), depth + 1));
                        }
                    }
                }
            }
            result = visited;
        }

        // 3. Max nodes cap
        if let Some(max_nodes) = config.max_nodes {
            if result.len() > max_nodes {
                let truncated: BTreeSet<String> = result.into_iter().take(max_nodes).collect();
                return truncated;
            }
        }

        result
    }

    fn render_class_diagram(
        classes: &BTreeSet<String>,
        subclasses: &BTreeSet<(String, String)>,
        equivs: &BTreeSet<(String, String)>,
        obj_props: &BTreeMap<String, (String, String)>,
        data_props: &BTreeMap<String, Vec<(String, String)>>,
        config: &MermaidConfig,
        prefixes: &BTreeMap<String, String>,
    ) -> Result<String> {
        let mut out = String::new();
        out.push_str("classDiagram\n");

        for c in classes {
            let compact = Self::compact_iri(c, prefixes);
            let id = Self::sanitize_id(&compact);

            out.push_str(&format!("    class {}[\"{}\"] {{\n", id, compact));
            if config.include_stereotypes {
                out.push_str("        <<owl:Class>>\n");
            }
            if config.include_data_properties {
                if let Some(props) = data_props.get(c) {
                    for (prop_name, datatype) in props {
                        out.push_str(&format!("        +{} {}\n", datatype, prop_name));
                    }
                }
            }
            out.push_str("    }\n");
        }
        out.push('\n');

        // Subclass relations: Sup <|-- Sub : rdfs:subClassOf
        for (sub, sup) in subclasses {
            let sub_id = Self::sanitize_id(&Self::compact_iri(sub, prefixes));
            let sup_id = Self::sanitize_id(&Self::compact_iri(sup, prefixes));
            out.push_str(&format!("    {} <|-- {} : rdfs:subClassOf\n", sup_id, sub_id));
        }

        // Equivalent class relations: C1 <..> C2 : owl:equivalentClass
        for (c1, c2) in equivs {
            let id1 = Self::sanitize_id(&Self::compact_iri(c1, prefixes));
            let id2 = Self::sanitize_id(&Self::compact_iri(c2, prefixes));
            out.push_str(&format!("    {} <..> {} : owl:equivalentClass\n", id1, id2));
        }

        // Object property relations: Domain --> Range : propName (owl:ObjectProperty)
        for (op, (dom, rng)) in obj_props {
            let dom_id = Self::sanitize_id(&Self::compact_iri(dom, prefixes));
            let rng_id = Self::sanitize_id(&Self::compact_iri(rng, prefixes));
            let prop_compact = Self::compact_iri(op, prefixes);
            out.push_str(&format!("    {} --> {} : {} (owl:ObjectProperty)\n", dom_id, rng_id, prop_compact));
        }

        Ok(out)
    }

    fn render_er_diagram(
        classes: &BTreeSet<String>,
        subclasses: &BTreeSet<(String, String)>,
        equivs: &BTreeSet<(String, String)>,
        obj_props: &BTreeMap<String, (String, String)>,
        data_props: &BTreeMap<String, Vec<(String, String)>>,
        config: &MermaidConfig,
        prefixes: &BTreeMap<String, String>,
    ) -> Result<String> {
        let mut out = String::new();
        out.push_str("erDiagram\n");

        for c in classes {
            let compact = Self::compact_iri(c, prefixes);
            let id = Self::sanitize_id(&compact);

            out.push_str(&format!("    {} {{\n", id));
            out.push_str(&format!("        string iri \"{}\"\n", compact));
            if config.include_data_properties {
                if let Some(props) = data_props.get(c) {
                    for (prop_name, datatype) in props {
                        let clean_prop = Self::sanitize_id(prop_name);
                        out.push_str(&format!("        {} {}\n", datatype.replace(':', "_"), clean_prop));
                    }
                }
            }
            out.push_str("    }\n");
        }
        out.push('\n');

        // Subclass relations in ER
        for (sub, sup) in subclasses {
            let sub_id = Self::sanitize_id(&Self::compact_iri(sub, prefixes));
            let sup_id = Self::sanitize_id(&Self::compact_iri(sup, prefixes));
            out.push_str(&format!("    {} ||--o{{ {} : rdfs_subClassOf\n", sup_id, sub_id));
        }

        // Equivalent classes in ER
        for (c1, c2) in equivs {
            let id1 = Self::sanitize_id(&Self::compact_iri(c1, prefixes));
            let id2 = Self::sanitize_id(&Self::compact_iri(c2, prefixes));
            out.push_str(&format!("    {} .. {} : owl_equivalentClass\n", id1, id2));
        }

        // Object properties in ER
        for (op, (dom, rng)) in obj_props {
            let dom_id = Self::sanitize_id(&Self::compact_iri(dom, prefixes));
            let rng_id = Self::sanitize_id(&Self::compact_iri(rng, prefixes));
            let prop_clean = Self::sanitize_id(&Self::compact_iri(op, prefixes));
            out.push_str(&format!("    {} ||--o{{ {} : {}\n", dom_id, rng_id, prop_clean));
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_class_diagram_translation() {
        let mut ontology = SetOntology::<ArcStr>::new();
        let b = Build::new();

        let dev_c = b.class("https://saref.etsi.org/core/Device");
        let sensor_c = b.class("http://www.w3.org/ns/sosa/Sensor");
        let sys_c = b.class("http://www.w3.org/ns/sosa/System");
        let is_hosted_by_op = b.object_property("http://www.w3.org/ns/sosa/isHostedBy");

        ontology.insert(DeclareClass(dev_c.clone()));
        ontology.insert(DeclareClass(sensor_c.clone()));
        ontology.insert(DeclareClass(sys_c.clone()));

        ontology.insert(SubClassOf {
            sub: ClassExpression::Class(sensor_c.clone()),
            sup: ClassExpression::Class(dev_c.clone()),
        });
        ontology.insert(ObjectPropertyDomain {
            ope: ObjectPropertyExpression::ObjectProperty(is_hosted_by_op.clone()),
            ce: ClassExpression::Class(sensor_c.clone()),
        });
        ontology.insert(ObjectPropertyRange {
            ope: ObjectPropertyExpression::ObjectProperty(is_hosted_by_op),
            ce: ClassExpression::Class(sys_c),
        });

        let config = MermaidConfig::default();
        let mermaid = MermaidTranslator::translate(&ontology, &config).unwrap();

        assert!(mermaid.contains("classDiagram"));
        assert!(mermaid.contains("saref_Device"));
        assert!(mermaid.contains("sosa_Sensor"));
        assert!(mermaid.contains("saref_Device <|-- sosa_Sensor : rdfs:subClassOf"));
        assert!(mermaid.contains("sosa_Sensor --> sosa_System : sosa:isHostedBy (owl:ObjectProperty)"));
    }

    #[test]
    fn test_er_diagram_translation() {
        let mut ontology = SetOntology::<ArcStr>::new();
        let b = Build::new();

        let c1 = b.class("http://example.org/A");
        let c2 = b.class("http://example.org/B");

        ontology.insert(DeclareClass(c1.clone()));
        ontology.insert(DeclareClass(c2.clone()));
        ontology.insert(SubClassOf {
            sub: ClassExpression::Class(c2),
            sup: ClassExpression::Class(c1),
        });

        let config = MermaidConfig {
            diagram_type: MermaidDiagramType::ErDiagram,
            ..Default::default()
        };

        let mermaid = MermaidTranslator::translate(&ontology, &config).unwrap();

        assert!(mermaid.contains("erDiagram"));
        assert!(mermaid.contains("http___example_org_A ||--o{ http___example_org_B : rdfs_subClassOf"));
    }
}

