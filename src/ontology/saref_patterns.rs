use horned_owl::model::*;
use horned_owl::ontology::set::SetOntology;
use serde::{Deserialize, Serialize};

pub const SAREF_CORE_IRI: &str = "https://saref.etsi.org/core/";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SarefPattern {
    FeatureOfInterest,
    Measurement,
    CommandFunction,
    SystemTopology,
    StateCommodity,
}

impl SarefPattern {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "feature_of_interest" | "feature" | "foi" => Some(Self::FeatureOfInterest),
            "measurement" | "observation" => Some(Self::Measurement),
            "command_function" | "command" | "function" => Some(Self::CommandFunction),
            "system_topology" | "system" | "topology" => Some(Self::SystemTopology),
            "state_commodity" | "state" | "commodity" => Some(Self::StateCommodity),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::FeatureOfInterest => "feature_of_interest",
            Self::Measurement => "measurement",
            Self::CommandFunction => "command_function",
            Self::SystemTopology => "system_topology",
            Self::StateCommodity => "state_commodity",
        }
    }
}

pub struct SarefPatternRegistry;

impl SarefPatternRegistry {
    /// Injects SAREF design pattern baseline classes and properties into an existing horned-owl SetOntology graph.
    pub fn apply_pattern(pattern: &SarefPattern, ontology: &mut SetOntology<ArcStr>) {
        let build = Build::new();
        let core_prefix = SAREF_CORE_IRI;

        match pattern {
            SarefPattern::FeatureOfInterest => {
                let foi_class = build.class(build.iri(format!("{}FeatureOfInterest", core_prefix).as_str()));
                let prop_class = build.class(build.iri(format!("{}Property", core_prefix).as_str()));

                ontology.insert(DeclareClass(foi_class.clone()));
                ontology.insert(DeclareClass(prop_class.clone()));

                let has_prop = build.object_property(build.iri(format!("{}hasProperty", core_prefix).as_str()));
                let is_prop_of = build.object_property(build.iri(format!("{}isPropertyOf", core_prefix).as_str()));

                ontology.insert(DeclareObjectProperty(has_prop.clone()));
                ontology.insert(DeclareObjectProperty(is_prop_of.clone()));

                // Domains & Ranges
                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(has_prop.clone()),
                    ce: ClassExpression::Class(foi_class.clone()),
                });
                ontology.insert(ObjectPropertyRange {
                    ope: ObjectPropertyExpression::ObjectProperty(has_prop),
                    ce: ClassExpression::Class(prop_class.clone()),
                });

                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(is_prop_of.clone()),
                    ce: ClassExpression::Class(prop_class),
                });
                ontology.insert(ObjectPropertyRange {
                    ope: ObjectPropertyExpression::ObjectProperty(is_prop_of),
                    ce: ClassExpression::Class(foi_class),
                });
            }
            SarefPattern::Measurement => {
                let meas_class = build.class(build.iri(format!("{}Measurement", core_prefix).as_str()));
                let prop_class = build.class(build.iri(format!("{}Property", core_prefix).as_str()));
                let dev_class = build.class(build.iri(format!("{}Device", core_prefix).as_str()));
                let unit_class = build.class(build.iri(format!("{}UnitOfMeasure", core_prefix).as_str()));

                ontology.insert(DeclareClass(meas_class.clone()));
                ontology.insert(DeclareClass(prop_class.clone()));
                ontology.insert(DeclareClass(dev_class.clone()));
                ontology.insert(DeclareClass(unit_class.clone()));

                let makes_meas = build.object_property(build.iri(format!("{}makesMeasurement", core_prefix).as_str()));
                let relates_prop = build.object_property(build.iri(format!("{}relatesToProperty", core_prefix).as_str()));
                let is_meas_in = build.object_property(build.iri(format!("{}isMeasuredIn", core_prefix).as_str()));

                ontology.insert(DeclareObjectProperty(makes_meas.clone()));
                ontology.insert(DeclareObjectProperty(relates_prop.clone()));
                ontology.insert(DeclareObjectProperty(is_meas_in.clone()));

                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(makes_meas),
                    ce: ClassExpression::Class(dev_class),
                });
                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(relates_prop.clone()),
                    ce: ClassExpression::Class(meas_class.clone()),
                });
                ontology.insert(ObjectPropertyRange {
                    ope: ObjectPropertyExpression::ObjectProperty(relates_prop),
                    ce: ClassExpression::Class(prop_class),
                });
                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(is_meas_in),
                    ce: ClassExpression::Class(meas_class.clone()),
                });

                // Data properties for Measurement value and timestamp
                let has_val = build.data_property(build.iri(format!("{}hasValue", core_prefix).as_str()));
                let has_time = build.data_property(build.iri(format!("{}hasTimestamp", core_prefix).as_str()));

                ontology.insert(DeclareDataProperty(has_val.clone()));
                ontology.insert(DeclareDataProperty(has_time.clone()));

                ontology.insert(DataPropertyDomain {
                    dp: has_val.clone(),
                    ce: ClassExpression::Class(meas_class.clone()),
                });
                let float_dt = build.datatype(build.iri("http://www.w3.org/2001/XMLSchema#float"));
                ontology.insert(DataPropertyRange {
                    dp: has_val,
                    dr: DataRange::Datatype(float_dt),
                });

                ontology.insert(DataPropertyDomain {
                    dp: has_time.clone(),
                    ce: ClassExpression::Class(meas_class),
                });
                let time_dt = build.datatype(build.iri("http://www.w3.org/2001/XMLSchema#dateTime"));
                ontology.insert(DataPropertyRange {
                    dp: has_time,
                    dr: DataRange::Datatype(time_dt),
                });
            }
            SarefPattern::CommandFunction => {
                let func_class = build.class(build.iri(format!("{}Function", core_prefix).as_str()));
                let cmd_class = build.class(build.iri(format!("{}Command", core_prefix).as_str()));
                let dev_class = build.class(build.iri(format!("{}Device", core_prefix).as_str()));

                ontology.insert(DeclareClass(func_class.clone()));
                ontology.insert(DeclareClass(cmd_class.clone()));
                ontology.insert(DeclareClass(dev_class.clone()));

                let has_func = build.object_property(build.iri(format!("{}hasFunction", core_prefix).as_str()));
                let has_cmd = build.object_property(build.iri(format!("{}hasCommand", core_prefix).as_str()));
                let acts_upon = build.object_property(build.iri(format!("{}actsUpon", core_prefix).as_str()));

                ontology.insert(DeclareObjectProperty(has_func.clone()));
                ontology.insert(DeclareObjectProperty(has_cmd.clone()));
                ontology.insert(DeclareObjectProperty(acts_upon.clone()));

                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(has_func),
                    ce: ClassExpression::Class(dev_class),
                });
                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(has_cmd.clone()),
                    ce: ClassExpression::Class(func_class.clone()),
                });
                ontology.insert(ObjectPropertyRange {
                    ope: ObjectPropertyExpression::ObjectProperty(has_cmd),
                    ce: ClassExpression::Class(cmd_class),
                });
                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(acts_upon),
                    ce: ClassExpression::Class(func_class),
                });
            }
            SarefPattern::SystemTopology => {
                let sys_class = build.class(build.iri(format!("{}System", core_prefix).as_str()));

                ontology.insert(DeclareClass(sys_class.clone()));

                let has_subsys = build.object_property(build.iri(format!("{}hasSubsystem", core_prefix).as_str()));
                let connects_to = build.object_property(build.iri(format!("{}connectsTo", core_prefix).as_str()));

                ontology.insert(DeclareObjectProperty(has_subsys.clone()));
                ontology.insert(DeclareObjectProperty(connects_to.clone()));

                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(has_subsys.clone()),
                    ce: ClassExpression::Class(sys_class.clone()),
                });
                ontology.insert(ObjectPropertyRange {
                    ope: ObjectPropertyExpression::ObjectProperty(has_subsys),
                    ce: ClassExpression::Class(sys_class.clone()),
                });

                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(connects_to.clone()),
                    ce: ClassExpression::Class(sys_class.clone()),
                });
                ontology.insert(ObjectPropertyRange {
                    ope: ObjectPropertyExpression::ObjectProperty(connects_to),
                    ce: ClassExpression::Class(sys_class),
                });
            }
            SarefPattern::StateCommodity => {
                let state_class = build.class(build.iri(format!("{}State", core_prefix).as_str()));
                let comm_class = build.class(build.iri(format!("{}Commodity", core_prefix).as_str()));
                let dev_class = build.class(build.iri(format!("{}Device", core_prefix).as_str()));

                ontology.insert(DeclareClass(state_class.clone()));
                ontology.insert(DeclareClass(comm_class.clone()));
                ontology.insert(DeclareClass(dev_class.clone()));

                let has_state = build.object_property(build.iri(format!("{}hasState", core_prefix).as_str()));
                let consumes = build.object_property(build.iri(format!("{}isConsumedBy", core_prefix).as_str()));

                ontology.insert(DeclareObjectProperty(has_state.clone()));
                ontology.insert(DeclareObjectProperty(consumes.clone()));

                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(has_state),
                    ce: ClassExpression::Class(dev_class),
                });
                ontology.insert(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(consumes),
                    ce: ClassExpression::Class(comm_class),
                });
            }
        }
    }

    /// Applies a list of patterns to the given ontology graph.
    pub fn apply_patterns(patterns: &[SarefPattern], ontology: &mut SetOntology<ArcStr>) {
        for pattern in patterns {
            Self::apply_pattern(pattern, ontology);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_saref_patterns() {
        let mut ont = SetOntology::new();
        SarefPatternRegistry::apply_patterns(
            &[SarefPattern::FeatureOfInterest, SarefPattern::Measurement],
            &mut ont,
        );
        let count = ont.iter().count();
        assert!(count >= 10, "Expected at least 10 axioms after applying FeatureOfInterest and Measurement patterns");
    }
}
