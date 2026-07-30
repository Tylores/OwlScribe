use crate::ontology::mcguinness_builder::{
    ClassDefinition, ClassMapping, DataPropertyDefinition, IndividualDefinition,
    ObjectPropertyDefinition, PropertyMapping,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StagedOntologyInventory {
    pub classes: Vec<ClassDefinition>,
    pub object_properties: Vec<ObjectPropertyDefinition>,
    pub data_properties: Vec<DataPropertyDefinition>,
    pub individuals: Vec<IndividualDefinition>,
    pub class_mappings: Vec<ClassMapping>,
    pub property_mappings: Vec<PropertyMapping>,
    pub staged_sections: Vec<String>,
    #[serde(default)]
    pub saref_patterns: Vec<String>,
}

impl StagedOntologyInventory {
    fn session_file_path() -> PathBuf {
        std::env::temp_dir().join("owlscribe_staged_inventory.json")
    }

    pub fn load_session() -> Self {
        let path = Self::session_file_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(inv) = serde_json::from_str::<StagedOntologyInventory>(&content) {
                    return inv;
                }
            }
        }
        StagedOntologyInventory::default()
    }

    pub fn save_session(&self) {
        let path = Self::session_file_path();
        if let Ok(json_str) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json_str);
        }
    }

    pub fn clear(&mut self) {
        self.classes.clear();
        self.object_properties.clear();
        self.data_properties.clear();
        self.individuals.clear();
        self.class_mappings.clear();
        self.property_mappings.clear();
        self.staged_sections.clear();
        self.saref_patterns.clear();
        self.save_session();
    }

    pub fn add_classes(&mut self, new_classes: Vec<ClassDefinition>) {
        for c in new_classes {
            if let Some(existing) = self.classes.iter_mut().find(|x| x.name.eq_ignore_ascii_case(&c.name)) {
                *existing = c;
            } else {
                self.classes.push(c);
            }
        }
        self.save_session();
    }

    pub fn add_object_properties(&mut self, new_props: Vec<ObjectPropertyDefinition>) {
        for p in new_props {
            if let Some(existing) = self.object_properties.iter_mut().find(|x| x.name.eq_ignore_ascii_case(&p.name)) {
                *existing = p;
            } else {
                self.object_properties.push(p);
            }
        }
        self.save_session();
    }

    pub fn add_data_properties(&mut self, new_props: Vec<DataPropertyDefinition>) {
        for p in new_props {
            if let Some(existing) = self.data_properties.iter_mut().find(|x| x.name.eq_ignore_ascii_case(&p.name)) {
                *existing = p;
            } else {
                self.data_properties.push(p);
            }
        }
        self.save_session();
    }

    pub fn add_individuals(&mut self, new_indivs: Vec<IndividualDefinition>) {
        for ind in new_indivs {
            if let Some(existing) = self.individuals.iter_mut().find(|x| x.name.eq_ignore_ascii_case(&ind.name)) {
                *existing = ind;
            } else {
                self.individuals.push(ind);
            }
        }
        self.save_session();
    }

    pub fn add_class_mappings(&mut self, new_mappings: Vec<ClassMapping>) {
        for m in new_mappings {
            if let Some(existing) = self.class_mappings.iter_mut().find(|x| x.term.eq_ignore_ascii_case(&m.term)) {
                *existing = m;
            } else {
                self.class_mappings.push(m);
            }
        }
        self.save_session();
    }

    pub fn add_property_mappings(&mut self, new_mappings: Vec<PropertyMapping>) {
        for m in new_mappings {
            if let Some(existing) = self.property_mappings.iter_mut().find(|x| x.property_name.eq_ignore_ascii_case(&m.property_name)) {
                *existing = m;
            } else {
                self.property_mappings.push(m);
            }
        }
        self.save_session();
    }

    pub fn add_saref_patterns(&mut self, patterns: Vec<String>) {
        for p in patterns {
            if !self.saref_patterns.iter().any(|existing| existing.eq_ignore_ascii_case(&p)) {
                self.saref_patterns.push(p);
            }
        }
        self.save_session();
    }
}

pub static STAGED_INVENTORY: LazyLock<Mutex<StagedOntologyInventory>> =
    LazyLock::new(|| Mutex::new(StagedOntologyInventory::load_session()));
