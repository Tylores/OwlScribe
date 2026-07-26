use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecType {
    Iso,
    Ieee,
    W3c,
    Nist,
    Rfc,
    Auto,
}

impl Default for SpecType {
    fn default() -> Self {
        SpecType::Auto
    }
}

#[derive(Debug, Clone)]
pub struct SpecProfile {
    pub spec_type: SpecType,
    pub terms_section_titles: Vec<&'static str>,
    pub references_section_titles: Vec<&'static str>,
    pub conformance_section_titles: Vec<&'static str>,
}

impl SpecProfile {
    pub fn for_type(spec_type: SpecType) -> Self {
        match spec_type {
            SpecType::Iso => Self {
                spec_type: SpecType::Iso,
                terms_section_titles: vec![
                    "terms and definitions",
                    "terms, definitions and abbreviated terms",
                    "3 terms and definitions",
                    "3. terms and definitions",
                ],
                references_section_titles: vec![
                    "normative references",
                    "2 normative references",
                    "2. normative references",
                ],
                conformance_section_titles: vec!["conformance", "clause 5 conformance"],
            },
            SpecType::Ieee => Self {
                spec_type: SpecType::Ieee,
                terms_section_titles: vec![
                    "definitions",
                    "definitions, acronyms, and abbreviations",
                    "3. definitions",
                    "clause 3",
                ],
                references_section_titles: vec![
                    "normative references",
                    "2. references",
                    "clause 2",
                ],
                conformance_section_titles: vec!["conformance", "conformance requirements"],
            },
            SpecType::W3c => Self {
                spec_type: SpecType::W3c,
                terms_section_titles: vec![
                    "terminology",
                    "definitions",
                    "concepts",
                    "terms and definitions",
                ],
                references_section_titles: vec!["references", "normative references"],
                conformance_section_titles: vec!["conformance", "conformance requirements"],
            },
            SpecType::Nist => Self {
                spec_type: SpecType::Nist,
                terms_section_titles: vec![
                    "terms and definitions",
                    "definitions and terms",
                    "acronyms and abbreviations",
                ],
                references_section_titles: vec!["references", "normative references"],
                conformance_section_titles: vec!["conformance", "security requirements"],
            },
            SpecType::Rfc => Self {
                spec_type: SpecType::Rfc,
                terms_section_titles: vec![
                    "terminology",
                    "definitions",
                    "terms and definitions",
                ],
                references_section_titles: vec![
                    "normative references",
                    "references",
                ],
                conformance_section_titles: vec![
                    "conformance",
                    "requirements notation",
                ],
            },
            SpecType::Auto => Self::for_type(SpecType::Iso),
        }
    }

    pub fn detect_type(content: &str) -> SpecType {
        let lower = content.to_lowercase();
        if lower.contains("iso/") || lower.contains("iso ") || lower.contains("iso/iec") {
            SpecType::Iso
        } else if lower.contains("ieee std") || lower.contains("ieee standard") || lower.contains("ansi/ieee") {
            SpecType::Ieee
        } else if lower.contains("w3c recommendation") || lower.contains("w3c working draft") {
            SpecType::W3c
        } else if lower.contains("nist special publication") || lower.contains("nist sp") {
            SpecType::Nist
        } else if lower.contains("request for comments:") || lower.contains("rfc ") || lower.contains("rfc2119") {
            SpecType::Rfc
        } else {
            SpecType::Auto
        }
    }
}
