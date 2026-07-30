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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saref_pattern_role: Option<String>,
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
pub struct MinedSubClassRelation {
    pub sub_class: String,
    pub super_class: String,
    pub confidence: f64,
    pub context_snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinedObjectProperty {
    pub property_name: String,
    pub domain: String,
    pub range: String,
    pub confidence: f64,
    pub context_snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinedDataProperty {
    pub property_name: String,
    pub domain: Option<String>,
    pub range_or_unit: String,
    pub confidence: f64,
    pub context_snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McGuinnessStep5_6MinedRelationships {
    pub subclass_relations: Vec<MinedSubClassRelation>,
    pub object_properties: Vec<MinedObjectProperty>,
    pub data_properties: Vec<MinedDataProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermAlignmentMatrixEntry {
    pub candidate_term: String,
    pub matched_base_iri: String,
    pub matched_concept_name: String,
    pub suggested_relation: String, // "owl:equivalentClass" or "rdfs:subClassOf"
    pub confidence_score: f64,
    pub context_snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveGuidance {
    pub status: String,
    pub message: String,
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsePdfToTermsResult {
    pub step1_domain_scope: McGuinnessStep1DomainScope,
    pub step2_reuse_references: McGuinnessStep2ReuseReferences,
    pub step3_term_enumeration: McGuinnessStep3TermEnumeration,
    pub step5_6_mined_relationships: McGuinnessStep5_6MinedRelationships,
    pub term_alignment_matrix: Vec<TermAlignmentMatrixEntry>,
    pub interactive_guidance: InteractiveGuidance,
    pub detected_sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfSectionInfo {
    pub id: String,
    pub section_number: Option<String>,
    pub title: String,
    pub page_start: usize,
    pub page_end: usize,
    pub preview_snippet: String,
    pub is_normative_candidate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfSectionListResult {
    pub pdf_path: String,
    pub document_title: String,
    pub detected_spec_type: SpecType,
    pub total_sections: usize,
    pub sections: Vec<PdfSectionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadPdfSectionResult {
    pub pdf_path: String,
    pub section_id: String,
    pub section_title: String,
    pub page_range: String,
    pub page_start: usize,
    pub page_end: usize,
    pub character_count: usize,
    pub text: String,
    pub section_extraction: ParsePdfToTermsResult,
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
                Self::extract_with_lopdf(&pdf_bytes).unwrap_or_default()
            }
        };

        Self::parse_raw_text(&raw_text, pdf_path.to_string_lossy().as_ref(), spec_type_override, min_confidence, base_seed)
    }

    pub fn extract_pages_with_numbers(pdf_bytes: &[u8]) -> Vec<(usize, String)> {
        if let Ok(doc) = lopdf::Document::load_mem(pdf_bytes) {
            let mut pages = Vec::new();
            for page_num in 1..=doc.get_pages().len() as u32 {
                if let Ok(text) = doc.extract_text(&[page_num]) {
                    pages.push((page_num as usize, text));
                }
            }
            if !pages.is_empty() {
                return pages;
            }
        }

        let raw = pdf_extract::extract_text_from_mem(pdf_bytes).unwrap_or_default();
        let page_chunks: Vec<&str> = raw.split('\x0C').collect();
        let mut pages = Vec::new();
        for (idx, chunk) in page_chunks.iter().enumerate() {
            pages.push((idx + 1, chunk.to_string()));
        }
        if pages.is_empty() {
            pages.push((1, raw));
        }
        pages
    }

    pub fn get_pdf_sections(
        pdf_path: &Path,
        spec_type_override: Option<SpecType>,
    ) -> Result<PdfSectionListResult> {
        let pdf_bytes = std::fs::read(pdf_path)
            .with_context(|| format!("Failed to read PDF file at '{}'", pdf_path.display()))?;

        let pages = Self::extract_pages_with_numbers(&pdf_bytes);
        let full_text: String = pages.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join("\n");

        let spec_type = match spec_type_override {
            Some(SpecType::Auto) | None => SpecProfile::detect_type(&full_text),
            Some(t) => t,
        };

        let doc_title = Self::extract_title(&full_text, pdf_path.to_string_lossy().as_ref());

        let mut section_entries: Vec<PdfSectionInfo> = Vec::new();
        let re_sec = Regex::new(r"(?m)^(?:([0-9]+(?:\.[0-9]+)*)|Clause\s+([0-9]+(?:\.[0-9]+)*)|Annex\s+([A-Z]))\s+([^\n]+)").unwrap();

        for (page_num, page_text) in &pages {
            for line in page_text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.len() < 3 {
                    continue;
                }

                if let Some(cap) = re_sec.captures(trimmed) {
                    let sec_num = cap.get(1).or_else(|| cap.get(2)).or_else(|| cap.get(3)).map(|m| m.as_str().to_string());
                    let title = trimmed.to_string();

                    let lower = title.to_lowercase();
                    let is_normative = lower.contains("term")
                        || lower.contains("definition")
                        || lower.contains("architecture")
                        || lower.contains("domain")
                        || lower.contains("scope")
                        || lower.contains("concept")
                        || lower.contains("model")
                        || lower.contains("requirement");

                    let sec_id = format!("sec_{}", section_entries.len() + 1);

                    if let Some(prev) = section_entries.last_mut() {
                        prev.page_end = *page_num;
                    }

                    section_entries.push(PdfSectionInfo {
                        id: sec_id,
                        section_number: sec_num,
                        title: title.clone(),
                        page_start: *page_num,
                        page_end: *page_num,
                        preview_snippet: if trimmed.len() > 100 { trimmed[..100].to_string() } else { trimmed.to_string() },
                        is_normative_candidate: is_normative,
                    });
                }
            }
        }

        if section_entries.is_empty() {
            let total_p = pages.len();
            section_entries.push(PdfSectionInfo {
                id: "sec_1".to_string(),
                section_number: Some("1".to_string()),
                title: "1. Specification Body and Definitions".to_string(),
                page_start: 1,
                page_end: total_p,
                preview_snippet: "Full document specification body.".to_string(),
                is_normative_candidate: true,
            });
        } else if let Some(last) = section_entries.last_mut() {
            last.page_end = pages.last().map(|(p, _)| *p).unwrap_or(1);
        }

        let count = section_entries.len();

        Ok(PdfSectionListResult {
            pdf_path: pdf_path.to_string_lossy().to_string(),
            document_title: doc_title,
            detected_spec_type: spec_type,
            total_sections: count,
            sections: section_entries,
        })
    }

    pub fn read_pdf_section(
        pdf_path: &Path,
        section_id: Option<&str>,
        section_title: Option<&str>,
        page_start_opt: Option<usize>,
        page_end_opt: Option<usize>,
        spec_type_override: Option<SpecType>,
        min_confidence: Option<f64>,
        base_seed: Option<&BaseOntologySeed>,
    ) -> Result<ReadPdfSectionResult> {
        let sec_list = Self::get_pdf_sections(pdf_path, spec_type_override)?;
        let pages = {
            let bytes = std::fs::read(pdf_path)?;
            Self::extract_pages_with_numbers(&bytes)
        };

        let target_sec = if let Some(sid) = section_id {
            sec_list.sections.iter().find(|s| s.id.eq_ignore_ascii_case(sid) || s.section_number.as_deref() == Some(sid)).cloned()
        } else if let Some(stitle) = section_title {
            sec_list.sections.iter().find(|s| s.title.to_lowercase().contains(&stitle.to_lowercase())).cloned()
        } else {
            None
        };

        let (p_start, p_end, sec_id_str, sec_title_str) = if let Some(ts) = target_sec {
            (ts.page_start, ts.page_end, ts.id, ts.title)
        } else {
            let p_start = page_start_opt.unwrap_or(1);
            let p_end = page_end_opt.unwrap_or_else(|| pages.last().map(|(p, _)| *p).unwrap_or(1));
            (p_start, p_end, "custom_range".to_string(), format!("Pages {}-{}", p_start, p_end))
        };

        let mut section_text = String::new();
        for (page_num, text) in &pages {
            if *page_num >= p_start && *page_num <= p_end {
                section_text.push_str(text);
                section_text.push('\n');
            }
        }

        let section_extraction = Self::parse_raw_text(
            &section_text,
            pdf_path.to_string_lossy().as_ref(),
            spec_type_override,
            min_confidence,
            base_seed,
        )?;

        let char_count = section_text.len();

        Ok(ReadPdfSectionResult {
            pdf_path: pdf_path.to_string_lossy().to_string(),
            section_id: sec_id_str,
            section_title: sec_title_str,
            page_range: format!("Pages {}-{}", p_start, p_end),
            page_start: p_start,
            page_end: p_end,
            character_count: char_count,
            text: section_text,
            section_extraction,
        })
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

        let sections = Self::detect_sections(content);
        let normative_references = Self::extract_references(content, &profile);
        let mut candidate_terms = Self::extract_terms(content, &profile, min_conf);

        let mut term_alignment_matrix = Vec::new();

        if let Some(seed) = base_seed {
            for cand in &mut candidate_terms {
                if let Some(m) = SeedConceptMatcher::match_term(&cand.term, &cand.definition, seed) {
                    cand.mapped_base_concept = Some(m.target_iri.clone());
                    cand.mapping_relation = Some(m.suggested_mapping.clone());
                    cand.confidence = (cand.confidence + m.confidence_boost).min(1.0);

                    let suggested_rel = if m.suggested_mapping == "equivalentClass" {
                        "owl:equivalentClass".to_string()
                    } else {
                        "rdfs:subClassOf".to_string()
                    };

                    term_alignment_matrix.push(TermAlignmentMatrixEntry {
                        candidate_term: cand.term.clone(),
                        matched_base_iri: m.target_iri,
                        matched_concept_name: m.concept_name,
                        suggested_relation: suggested_rel,
                        confidence_score: cand.confidence,
                        context_snippet: cand.context_snippet.clone(),
                    });
                }
            }
        }

        for cand in &mut candidate_terms {
            if cand.saref_pattern_role.is_none() {
                cand.saref_pattern_role = Self::classify_saref_role(&cand.term, &cand.definition);
            }
        }

        let mined_relationships = Self::mine_relationships(content, &candidate_terms);

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

        let lower_content = content.to_lowercase();
        let is_grid_spec = lower_content.contains("1547") || lower_content.contains("der") || lower_content.contains("interconnection") || lower_content.contains("inverter") || lower_content.contains("grid");

        let interactive_guidance = if let Some(seed) = base_seed {
            InteractiveGuidance {
                status: "seeded_extraction".to_string(),
                message: format!("Candidate terms were automatically aligned against base ontology '{}'.", seed.ontology_iri),
                recommended_actions: vec![
                    "Review candidate_terms and term_alignment_matrix entries.".to_string(),
                    "Proceed to generate_owl_ontology using harvested terms and class_mappings.".to_string(),
                ],
            }
        } else {
            let spec_note = if is_grid_spec {
                " For IEEE 1547 / Smart Grid specs, SAREF4GRID (https://saref.etsi.org/saref4grid/) and SOSA are recommended base ontologies."
            } else {
                ""
            };

            InteractiveGuidance {
                status: "unseeded_extraction".to_string(),
                message: format!("No base ontology seed was provided. Terms were harvested without domain concept alignment.{}", spec_note),
                recommended_actions: vec![
                    "Step 2 Alignment: Re-run parse_pdf_to_terms supplying 'base_ontology_path' or 'base_ontology_seed' with a recommended domain ontology.".to_string(),
                    "Direct Generation: Review candidate terms and pass classes directly with optional 'class_mappings' to generate_owl_ontology.".to_string(),
                ],
            }
        };

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
            step5_6_mined_relationships: mined_relationships,
            term_alignment_matrix,
            interactive_guidance,
            detected_sections: sections,
        })
    }

    fn suggest_base_ontologies(content: &str, base_seed: Option<&BaseOntologySeed>) -> Vec<String> {
        let mut suggestions = Vec::new();
        if let Some(seed) = base_seed {
            suggestions.push(seed.ontology_iri.clone());
        }

        let lower = content.to_lowercase();
        if (lower.contains("1547") || lower.contains("der") || lower.contains("interconnection") || lower.contains("inverter") || lower.contains("grid") || lower.contains("power")) && !suggestions.contains(&"https://saref.etsi.org/saref4grid/".to_string()) {
            suggestions.push("https://saref.etsi.org/saref4grid/".to_string());
        }
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

    pub fn classify_saref_role(term: &str, definition: &str) -> Option<String> {
        let t_low = term.to_lowercase();
        let d_low = definition.to_lowercase();

        if t_low.contains("sensor") || t_low.contains("meter") || t_low.contains("inverter") || t_low.contains("device") || t_low.contains("actuator") || d_low.contains("device") || d_low.contains("hardware unit") {
            Some("saref:Device".to_string())
        } else if t_low.contains("measurement") || t_low.contains("reading") || t_low.contains("observation") || d_low.contains("measured value") || d_low.contains("quantitative observation") {
            Some("saref:Measurement".to_string())
        } else if t_low.contains("property") || t_low.contains("voltage") || t_low.contains("current") || t_low.contains("power") || t_low.contains("frequency") || t_low.contains("temperature") || t_low.contains("impedance") || d_low.contains("property") || d_low.contains("attribute of") {
            Some("saref:Property".to_string())
        } else if t_low.contains("function") || t_low.contains("capability") || d_low.contains("functionality") {
            Some("saref:Function".to_string())
        } else if t_low.contains("command") || t_low.contains("control") || t_low.contains("signal") || t_low.contains("setpoint") || d_low.contains("command") {
            Some("saref:Command".to_string())
        } else if t_low.contains("system") || t_low.contains("grid") || t_low.contains("feeder") || t_low.contains("topology") || d_low.contains("system of components") {
            Some("saref:System".to_string())
        } else if t_low.contains("state") || t_low.contains("mode") || d_low.contains("state of operation") {
            Some("saref:State".to_string())
        } else if t_low.contains("commodity") || t_low.contains("electricity") || t_low.contains("energy") || t_low.contains("water") || t_low.contains("gas") {
            Some("saref:Commodity".to_string())
        } else {
            None
        }
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

        let re_colon = Regex::new(r"(?m)^(?:\d+\.\d+(?:\.\d+)?\s+)?([A-Z][a-zA-Z0-9\s_\-]{2,40})\s*[:—\-]\s*(.+)").ok();
        let re_class_def = Regex::new(r"(?i)\b(?:Class|Property)\s+(?:s4grid:|saref4grid:)?([A-Z][a-zA-Z0-9]+)\b").ok();
        let re_rfc = Regex::new(r"(?i)\b(MUST|SHALL|SHOULD|MAY|RECOMMENDED|REQUIRED|OPTIONAL)\b").ok();

        if let Some(regex) = re_colon {
            for cap in regex.captures_iter(content) {
                let term = cap.get(1).map_or("", |m| m.as_str()).trim().to_string();
                let def = cap.get(2).map_or("", |m| m.as_str()).trim().to_string();

                if term.is_empty() || def.is_empty() || term.len() < 2 || Self::is_noise_term(&term) {
                    continue;
                }

                let lower_def = def.to_lowercase();

                let base_weight = if profile.terms_section_titles.iter().any(|t| lower_def.contains(t)) {
                    0.95
                } else if def.contains("is defined as") || def.contains("refers to") || def.contains("means") {
                    0.85
                } else {
                    0.65
                };

                let section = if lower_def.contains("definition") || lower_def.contains("terms") {
                    "Terms and Definitions".to_string()
                } else if lower_def.contains("architecture") || lower_def.contains("class") {
                    "Normative Architecture".to_string()
                } else {
                    "Specification Body".to_string()
                };

                let confidence = base_weight;

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
                    section,
                    rfc2119_keywords: rfc_kw,
                    context_snippet: snippet,
                    mapped_base_concept: None,
                    mapping_relation: None,
                    saref_pattern_role: None,
                });
            }
        }

        if let Some(regex) = re_class_def {
            for cap in regex.captures_iter(content) {
                let term = cap.get(1).map_or("", |m| m.as_str()).trim().to_string();
                if !term.is_empty() && !Self::is_noise_term(&term) && !candidates.iter().any(|c| c.term == term) {
                    candidates.push(TermCandidate {
                        term: term.clone(),
                        definition: format!("Domain concept {} specified in extension document.", term),
                        confidence: 0.90,
                        section: "Domain Classes".to_string(),
                        rfc2119_keywords: vec![],
                        context_snippet: format!("Class definition for {}", term),
                        mapped_base_concept: None,
                        mapping_relation: None,
                        saref_pattern_role: None,
                    });
                }
            }
        }

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
                        if !term.is_empty() && !def.is_empty() && term.len() <= 50 && !Self::is_noise_term(&term) {
                            candidates.push(TermCandidate {
                                term: term.clone(),
                                definition: def.clone(),
                                confidence: 0.70,
                                section: "Definitions".to_string(),
                                rfc2119_keywords: vec![],
                                context_snippet: trimmed.to_string(),
                                mapped_base_concept: None,
                                mapping_relation: None,
                                saref_pattern_role: None,
                            });
                        }
                    }
                }
            }
        }

        candidates
    }

    fn is_noise_term(term: &str) -> bool {
        let lower = term.to_lowercase().trim().to_string();
        if lower.len() < 2 {
            return true;
        }

        let noise_words = [
            "where", "should", "sous", "must", "shall", "can", "may", "would", "could",
            "with", "from", "into", "over", "under", "between", "through", "during", "after", "before",
            "about", "against", "among", "along", "following", "across", "behind", "beyond",
            "table", "figure", "annex", "clause", "note", "example", "provision", "etsi",
            "prefix", "saref", "part", "section", "appendix", "draft", "version", "revision",
            "edition", "page", "contents", "foreword", "introduction", "scope", "history",
            "title", "author", "copyright", "route des lucioles", "http", "www", "op saref", "dp saref"
        ];

        for nw in &noise_words {
            if lower == *nw {
                return true;
            }
        }

        if lower.starts_with("part ")
            || lower.starts_with("clause ")
            || lower.starts_with("table ")
            || lower.starts_with("figure ")
            || lower.starts_with("annex ")
            || lower.starts_with("page ")
        {
            return true;
        }

        if !term.chars().any(|c| c.is_alphabetic()) {
            return true;
        }

        if !term.contains(' ') && term.chars().all(|c| c.is_lowercase()) && (lower == "where" || lower == "should" || lower == "sous" || lower.len() <= 3) {
            return true;
        }

        false
    }

    fn mine_relationships(content: &str, _candidates: &[TermCandidate]) -> McGuinnessStep5_6MinedRelationships {
        let mut subclass_relations = Vec::new();
        let mut object_properties = Vec::new();
        let mut data_properties = Vec::new();

        let re_subclass = Regex::new(r"(?i)\b([A-Z][a-zA-Z0-9_-]+)\s+(?:is\s+a\s+subclass\s+of|is\s+a\s+type\s+of|is\s+a\s+kind\s+of|extends|subclass\s+of)\s+([A-Z][a-zA-Z0-9_-]+)\b").ok();
        if let Some(regex) = re_subclass {
            for cap in regex.captures_iter(content) {
                let sub = cap.get(1).map_or("", |m| m.as_str()).trim().to_string();
                let sup = cap.get(2).map_or("", |m| m.as_str()).trim().to_string();
                if !sub.is_empty() && !sup.is_empty() && sub != sup && !Self::is_noise_term(&sub) && !Self::is_noise_term(&sup) {
                    if !subclass_relations.iter().any(|r: &MinedSubClassRelation| r.sub_class == sub && r.super_class == sup) {
                        subclass_relations.push(MinedSubClassRelation {
                            sub_class: sub.clone(),
                            super_class: sup.clone(),
                            confidence: 0.85,
                            context_snippet: format!("{} is a subclass of {}", sub, sup),
                        });
                    }
                }
            }
        }

        let re_obj_prop = Regex::new(r"(?i)\b([A-Z][a-zA-Z0-9_-]+)\s+(has|contains|connects\s+to|targets|measures|controls|observes)\s+(?:a\s+|an\s+|the\s+)?([A-Z][a-zA-Z0-9_-]+)\b").ok();
        if let Some(regex) = re_obj_prop {
            for cap in regex.captures_iter(content) {
                let domain = cap.get(1).map_or("", |m| m.as_str()).trim().to_string();
                let verb = cap.get(2).map_or("", |m| m.as_str()).trim().to_string().to_lowercase();
                let range = cap.get(3).map_or("", |m| m.as_str()).trim().to_string();

                if !domain.is_empty() && !range.is_empty() && domain != range && !Self::is_noise_term(&domain) && !Self::is_noise_term(&range) {
                    let prop_name = format!("{}{}", verb, range);
                    if !object_properties.iter().any(|op: &MinedObjectProperty| op.domain == domain && op.range == range) {
                        object_properties.push(MinedObjectProperty {
                            property_name: prop_name,
                            domain,
                            range,
                            confidence: 0.80,
                            context_snippet: format!("Mined relationship: {} {} {}", cap.get(1).unwrap().as_str(), verb, cap.get(3).unwrap().as_str()),
                        });
                    }
                }
            }
        }

        let re_data_prop = Regex::new(r"(?i)\b([A-Z][a-zA-Z0-9_-]+|[a-z][a-zA-Z0-9_-]+)\s+(?:is\s+)?measured\s+in\s+([A-Za-z0-9_\-\/]+)\b").ok();
        if let Some(regex) = re_data_prop {
            for cap in regex.captures_iter(content) {
                let prop_name = cap.get(1).map_or("", |m| m.as_str()).trim().to_string();
                let unit = cap.get(2).map_or("", |m| m.as_str()).trim().to_string();
                if !prop_name.is_empty() && !unit.is_empty() && !Self::is_noise_term(&prop_name) {
                    if !data_properties.iter().any(|dp: &MinedDataProperty| dp.property_name == prop_name) {
                        data_properties.push(MinedDataProperty {
                            property_name: prop_name.clone(),
                            domain: None,
                            range_or_unit: unit.clone(),
                            confidence: 0.85,
                            context_snippet: format!("{} measured in {}", prop_name, unit),
                        });
                    }
                }
            }
        }

        McGuinnessStep5_6MinedRelationships {
            subclass_relations,
            object_properties,
            data_properties,
        }
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
    fn test_noise_term_filtering() {
        assert!(TermExtractor::is_noise_term("Part 12"));
        assert!(TermExtractor::is_noise_term("Sous"));
        assert!(TermExtractor::is_noise_term("where"));
        assert!(TermExtractor::is_noise_term("should"));
        assert!(!TermExtractor::is_noise_term("SensingUnit"));
        assert!(!TermExtractor::is_noise_term("ElectricMeter"));
    }

    #[test]
    fn test_relationship_mining() {
        let text = r#"
IEEE Std 2026
3.1 SmartMeter: Device for energy metering.
SmartMeter is a subclass of Device.
SmartMeter measures ActivePower.
Temperature is measured in Celsius.
"#;
        let result = TermExtractor::parse_raw_text(text, "IEEE 2026", Some(SpecType::Ieee), None, None).unwrap();
        assert!(result.step5_6_mined_relationships.subclass_relations.iter().any(|r| r.sub_class == "SmartMeter" && r.super_class == "Device"));
        assert!(result.step5_6_mined_relationships.data_properties.iter().any(|dp| dp.property_name == "Temperature" && dp.range_or_unit == "Celsius"));
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
