use crate::ontology::base_ontology::{BaseOntologySeed, SeedConceptMatcher};
use crate::parser::spec_profile::{SpecProfile, SpecType};
use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermCandidate {
    pub term: String,
    pub definition: String,
    pub confidence: f64,
    pub section: String,
    pub rfc2119_keywords: Vec<String>,
    pub context_snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapped_base_concept: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping_relation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McGuinnessStep1DomainScope {
    pub document_title: String,
    pub detected_spec_type: SpecType,
    pub domain_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McGuinnessStep2ReuseReferences {
    pub normative_references: Vec<String>,
    pub candidate_ontologies: Vec<String>,
    pub suggested_base_ontologies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McGuinnessStep3TermEnumeration {
    pub total_terms_found: usize,
    pub term_candidates: Vec<TermCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsePdfToTermsResult {
    pub step1_domain_scope: McGuinnessStep1DomainScope,
    pub step2_reuse_references: McGuinnessStep2ReuseReferences,
    pub step3_term_enumeration: McGuinnessStep3TermEnumeration,
    pub detected_sections: Vec<String>,
}

pub struct TermExtractor;

impl TermExtractor {
    pub fn parse_pdf(
        pdf_path: &Path,
        spec_type_override: Option<SpecType>,
        min_confidence: Option<f64>,
        base_seed: Option<&BaseOntologySeed>,
    ) -> Result<ParsePdfToTermsResult> {
        let pdf_bytes = std::fs::read(pdf_path)
            .with_context(|| format!("Failed to read PDF file at '{}'", pdf_path.display()))?;

        let raw_text = match pdf_extract::extract_text_from_mem(&pdf_bytes) {
            Ok(text) => text,
            Err(_) => {
                // Fallback attempt with lopdf if pdf-extract fails
                Self::extract_with_lopdf(&pdf_bytes).unwrap_or_default()
            }
        };

        Self::parse_raw_text(&raw_text, pdf_path.to_string_lossy().as_ref(), spec_type_override, min_confidence, base_seed)
    }

    fn extract_with_lopdf(pdf_bytes: &[u8]) -> Result<String> {
        let doc = lopdf::Document::load_mem(pdf_bytes)?;
        let mut extracted_text = String::new();
        for page_num in 1..=doc.get_pages().len() as u32 {
            if let Ok(text) = doc.extract_text(&[page_num]) {
                extracted_text.push_str(&text);
                extracted_text.push('\n');
            }
        }
        Ok(extracted_text)
    }

    pub fn parse_raw_text(
        content: &str,
        doc_name: &str,
        spec_type_override: Option<SpecType>,
        min_confidence: Option<f64>,
        base_seed: Option<&BaseOntologySeed>,
    ) -> Result<ParsePdfToTermsResult> {
        let min_conf = min_confidence.unwrap_or(0.3);

        let spec_type = match spec_type_override {
            Some(SpecType::Auto) | None => SpecProfile::detect_type(content),
            Some(t) => t,
        };

        let profile = SpecProfile::for_type(spec_type);

        // Detect headings & sections
        let sections = Self::detect_sections(content);
        let normative_references = Self::extract_references(content, &profile);
        let mut candidate_terms = Self::extract_terms(content, &profile, min_conf);

        // Perform Seed Concept Alignment if base_seed is provided
        if let Some(seed) = base_seed {
            for cand in &mut candidate_terms {
                if let Some(m) = SeedConceptMatcher::match_term(&cand.term, &cand.definition, seed) {
                    cand.mapped_base_concept = Some(m.target_iri);
                    cand.mapping_relation = Some(m.suggested_mapping);
                    cand.confidence = (cand.confidence + m.confidence_boost).min(1.0);
                }
            }
        }

        let rfc_keywords = Self::extract_rfc2119_keywords(content);

        let doc_title = Self::extract_title(content, doc_name);
        let domain_scope = format!(
            "Specification ontology domain extracted from '{}' (Detected spec format: {:?}). Total {} terms harvested.",
            doc_title, spec_type, candidate_terms.len()
        );

        let mut candidate_ontologies = vec![
            "http://www.w3.org/2002/07/owl#".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
            "http://www.w3.org/2004/02/skos/core#".to_string(),
        ];
        if !rfc_keywords.is_empty() {
            candidate_ontologies.push("http://www.w3.org/ns/odrl/2/".to_string());
        }

        let suggested_base_ontologies = Self::suggest_base_ontologies(content, base_seed);

        Ok(ParsePdfToTermsResult {
            step1_domain_scope: McGuinnessStep1DomainScope {
                document_title: doc_title,
                detected_spec_type: spec_type,
                domain_scope,
            },
            step2_reuse_references: McGuinnessStep2ReuseReferences {
                normative_references,
                candidate_ontologies,
                suggested_base_ontologies,
            },
            step3_term_enumeration: McGuinnessStep3TermEnumeration {
                total_terms_found: candidate_terms.len(),
                term_candidates: candidate_terms,
            },
            detected_sections: sections,
        })
    }

    fn suggest_base_ontologies(content: &str, base_seed: Option<&BaseOntologySeed>) -> Vec<String> {
        let mut suggestions = Vec::new();
        if let Some(seed) = base_seed {
            suggestions.push(seed.ontology_iri.clone());
        }

        let lower = content.to_lowercase();
        if (lower.contains("sensor") || lower.contains("observation") || lower.contains("sensing")) && !suggestions.contains(&"http://www.w3.org/ns/sosa/".to_string()) {
            suggestions.push("http://www.w3.org/ns/sosa/".to_string());
        }
        if (lower.contains("device") || lower.contains("appliance") || lower.contains("saref")) && !suggestions.contains(&"http://saref.etsi.org/core/".to_string()) {
            suggestions.push("http://saref.etsi.org/core/".to_string());
        }
        if (lower.contains("unit") || lower.contains("quantity") || lower.contains("qudt")) && !suggestions.contains(&"http://qudt.org/2.1/schema/qudt".to_string()) {
            suggestions.push("http://qudt.org/2.1/schema/qudt".to_string());
        }

        suggestions
    }

    fn extract_title(content: &str, fallback: &str) -> String {
        for line in content.lines().take(15) {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.len() > 5 && !trimmed.starts_with('%') {
                return trimmed.to_string();
            }
        }
        fallback.to_string()
    }

    fn detect_sections(content: &str) -> Vec<String> {
        let mut sections = Vec::new();
        let re = Regex::new(r"(?m)^(?:[0-9]+\.|\bClause\s+[0-9]+\b|[A-Z][A-Za-z0-9\s]{3,40}:)\s*(.+)").ok();
        if let Some(regex) = re {
            for cap in regex.captures_iter(content).take(20) {
                if let Some(m) = cap.get(1) {
                    let sec = m.as_str().trim();
                    if !sec.is_empty() && !sections.contains(&sec.to_string()) {
                        sections.push(sec.to_string());
                    }
                }
            }
        }
        if sections.is_empty() {
            sections.push("Terms and Definitions".to_string());
            sections.push("Normative References".to_string());
        }
        sections
    }

    fn extract_references(content: &str, _profile: &SpecProfile) -> Vec<String> {
        let mut refs = Vec::new();
        let re_std = Regex::new(r"(?i)\b(ISO|IEC|IEEE|NIST|RFC|W3C|ANSI)\s*(?:Std\s*)?[\d\.-]+").ok();
        if let Some(regex) = re_std {
            for cap in regex.find_iter(content) {
                let found = cap.as_str().trim().to_string();
                if !refs.contains(&found) {
                    refs.push(found);
                }
            }
        }
        refs
    }

    fn extract_terms(content: &str, profile: &SpecProfile, min_confidence: f64) -> Vec<TermCandidate> {
        let mut candidates = Vec::new();

        // Pattern 1: ISO/IEEE style numbered terms: "3.1 term\n definition text"
        // Pattern 2: "Term: definition text" or "Term — definition text"
        let re_colon = Regex::new(r"(?m)^(?:\d+\.\d+\s+)?([A-Z][a-zA-Z0-9\s_\-]{2,40})\s*[:—\-]\s*(.+)").ok();
        let re_rfc = Regex::new(r"(?i)\b(MUST|SHALL|SHOULD|MAY|RECOMMENDED|REQUIRED|OPTIONAL)\b").ok();

        if let Some(regex) = re_colon {
            for cap in regex.captures_iter(content) {
                let term = cap.get(1).map_or("", |m| m.as_str()).trim().to_string();
                let def = cap.get(2).map_or("", |m| m.as_str()).trim().to_string();

                if term.is_empty() || def.is_empty() || term.len() < 2 {
                    continue;
                }

                let lower_def = def.to_lowercase();

                let confidence = if profile.terms_section_titles.iter().any(|t| lower_def.contains(t)) {
                    0.95
                } else if def.contains("is defined as") || def.contains("refers to") || def.contains("means") {
                    0.85
                } else {
                    0.65
                };

                if confidence < min_confidence {
                    continue;
                }

                let mut rfc_kw = Vec::new();
                if let Some(ref rfc_regex) = re_rfc {
                    for kw_match in rfc_regex.find_iter(&def) {
                        let kw = kw_match.as_str().to_uppercase();
                        if !rfc_kw.contains(&kw) {
                            rfc_kw.push(kw);
                        }
                    }
                }

                let snippet = if def.len() > 120 {
                    format!("{}...", &def[..120])
                } else {
                    def.clone()
                };

                candidates.push(TermCandidate {
                    term,
                    definition: def,
                    confidence,
                    section: "Terms and Definitions".to_string(),
                    rfc2119_keywords: rfc_kw,
                    context_snippet: snippet,
                    mapped_base_concept: None,
                    mapping_relation: None,
                });
            }
        }

        // If no term candidates matched colon regex, parse fallback line-by-line definitions
        if candidates.is_empty() {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.contains(" means ") || trimmed.contains(" is defined as ") {
                    let parts: Vec<&str> = if trimmed.contains(" means ") {
                        trimmed.splitn(2, " means ").collect()
                    } else {
                        trimmed.splitn(2, " is defined as ").collect()
                    };

                    if parts.len() == 2 {
                        let term = parts[0].trim().to_string();
                        let def = parts[1].trim().to_string();
                        if !term.is_empty() && !def.is_empty() && term.len() <= 50 {
                            candidates.push(TermCandidate {
                                term: term.clone(),
                                definition: def.clone(),
                                confidence: 0.70,
                                section: "Definitions".to_string(),
                                rfc2119_keywords: vec![],
                                context_snippet: trimmed.to_string(),
                                mapped_base_concept: None,
                                mapping_relation: None,
                            });
                        }
                    }
                }
            }
        }

        candidates
    }

    fn extract_rfc2119_keywords(content: &str) -> Vec<String> {
        let mut found = Vec::new();
        let keywords = ["MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", "OPTIONAL"];
        for kw in keywords {
            if content.contains(kw) && !found.contains(&kw.to_string()) {
                found.push(kw.to_string());
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso_spec_text() {
        let text = r#"
ISO 12345:2026(E)
1 Scope
This document specifies terms for software ontologies.

2 Normative references
ISO/IEC 27000 Information security.

3 Terms and definitions
3.1 Ontology: A formal, explicit specification of a shared conceptualization.
3.2 Axiom: A statement that is assumed to be true.
"#;
        let result = TermExtractor::parse_raw_text(text, "ISO 12345", Some(SpecType::Iso), None, None).unwrap();
        assert_eq!(result.step1_domain_scope.detected_spec_type, SpecType::Iso);
        assert!(!result.step3_term_enumeration.term_candidates.is_empty());
        let ontology_term = result.step3_term_enumeration.term_candidates.iter().find(|t| t.term == "Ontology");
        assert!(ontology_term.is_some());
        assert!(ontology_term.unwrap().definition.contains("formal, explicit specification"));
    }

    #[test]
    fn test_rfc2119_keywords_extraction() {
        let text = r#"
RFC 2119
1 Terminology
An endpoint MUST validate the OWL ontology payload.
An implementation SHOULD log warning messages.
"#;
        let result = TermExtractor::parse_raw_text(text, "RFC 2119", Some(SpecType::Rfc), None, None).unwrap();
        assert_eq!(result.step1_domain_scope.detected_spec_type, SpecType::Rfc);
        let term = result.step3_term_enumeration.term_candidates.first();
        if let Some(t) = term {
            assert!(t.rfc2119_keywords.contains(&"MUST".to_string()));
        }
    }
}
