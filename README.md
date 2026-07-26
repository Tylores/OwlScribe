# 🦉 OwlScribe: Specification PDF to OWL 2 Ontology MCP Server

**OwlScribe** is a high-performance Model Context Protocol (MCP) server written in Rust. It translates technical specification PDFs (ISO, IEEE, W3C, NIST, IETF RFCs) into standardized **OWL 2 ontologies** using native Rust AST primitives (`horned-owl`) and the **Noy & McGuinness 7-Step Ontology Development Methodology**.

Now supporting **Hybrid Domain Ontology Import**: seed PDF extraction with established base ontologies (e.g. SOSA, SSN, SAREF, QUDT) and perform post-extraction formal W3C graph binding (`owl:imports`, `owl:equivalentClass`, `rdfs:subClassOf`).

---

## 📋 Table of Contents
1. [Architecture & Design Principles](#-architecture--design-principles)
2. [Supported Specification Families](#-supported-specification-families)
3. [The McGuinness 7-Step Workflow](#-the-mcguinness-7-step-workflow)
4. [Hybrid Domain Ontology Integration](#-hybrid-domain-ontology-integration)
5. [MCP Tools Reference](#-mcp-tools-reference)
   - [`parse_pdf_to_terms`](#1-parse_pdf_to_terms)
   - [`generate_owl_ontology`](#2-generate_owl_ontology)
6. [Industry Use Cases](#-industry-use-cases)
7. [Best Practices & Recommendations](#-best-practices--recommendations)
8. [Installation & MCP Registration](#-installation--mcp-registration)
9. [Developer Quickstart & Testing](#-developer-quickstart--testing)

---

## 🏗️ Architecture & Design Principles

OwlScribe is architected as an asynchronous, zero-cost-abstraction Rust service using standard I/O (stdio) JSON-RPC 2.0 transport following the Model Context Protocol (2024-11-05) specification.

```
+-----------------------------------------------------------------------------------+
|                                  MCP Client (LLM / Agent)                         |
+-----------------------------------------------------------------------------------+
           |                                                       ^
     (1) JSON-RPC                                            (4) JSON-RPC
   parse_pdf_to_terms                                    generate_owl_ontology
 (with Base Ontology Seed)                             (with Graph Mappings & Imports)
           |                                                       |
           v                                                       v
+-----------------------+                               +---------------------------+
|   OwlScribe Parser    |                               |   OwlScribe Generator     |
|  - Spec Profiles      |                               |  - McGuinness Steps 4-7   |
|  - Term Harvester     |                               |  - Base Graph Merger      |
|  - Concept Matcher    |                               |  - Horned-Owl AST Builder |
|  - Base Recommender   |                               |  - Serializer (OFN/TTL)   |
+-----------------------+                               +---------------------------+
           |                                                       |
           v                                                       v
   McGuinness Steps 1-3                                  Validated OWL 2 Ontology
 (Domain, Scope, Seed Terms)                           (OFN / Turtle / RDF-XML)
```

### Key Architectural Strengths
- **Type-Safe AST Execution**: Uses `horned-owl` (v2.1) to guarantee that generated ontologies strictly satisfy W3C OWL 2 structural and semantic invariants.
- **Hybrid Base Ontology Seeding**: Ingests base domain ontologies (OWL Functional Syntax `.ofn` or JSON seeds) to ground PDF extraction terminology (e.g. mapping "Sensing Unit" to `sosa:Sensor`).
- **Post-Extraction Graph Binding**: Executes W3C `owl:imports`, `owl:equivalentClass`, and `rdfs:subClassOf` graph alignment post-harvesting.
- **Specification Profile Heuristics**: Custom section parsers for ISO, IEEE, W3C, NIST, and RFC documents to locate normative terminology clauses.
- **RFC 2119 Normative Keyword Extraction**: Automatically scans and tags requirement keywords (`MUST`, `SHALL`, `SHOULD`, `MAY`).
- **Multi-Format Serialization**: Supports OWL Functional Syntax (`.ofn`), Turtle (`.ttl`), and RDF/XML (`.owl`).

---

## 📑 Supported Specification Families

| Profile | Normative Term Section Titles | Reference Clauses | Special Features |
| :--- | :--- | :--- | :--- |
| **ISO** | `3 Terms and definitions`, `Terms, definitions and abbreviated terms` | `2 Normative references` | Clause 3.x numbered taxonomy parsing |
| **IEEE** | `3. Definitions`, `Definitions, acronyms, and abbreviations` | `2. References` | Clause 3.x term-colon definition matching |
| **W3C** | `Terminology`, `Definitions`, `Concepts` | `Normative references` | Web ontology class mapping suggestions |
| **NIST** | `Terms and definitions`, `Definitions and terms`, `Acronyms` | `References` | Security requirement level tagging |
| **RFC** | `1. Terminology`, `2. Definitions` | `Normative references` | RFC 2119 / RFC 8174 requirement extraction |

---

## 🧩 The McGuinness 7-Step Workflow

OwlScribe directly implements the **Noy & McGuinness Ontology Development 101** framework:

```
[Step 1: Domain & Scope]  --->  Detect document title, abstract, & target domain
          │
[Step 2: Reuse References] --->  Harvest normative references, base ontologies (OWL, RDFS, SKOS, SOSA), & suggestions
          │
[Step 3: Term Enumeration] --->  Extract term candidates, definitions, confidence scores, & seed mappings
          │                      (Output of parse_pdf_to_terms)
          │
          ▼  <-- User/LLM Confidence Refinement Phase -->
          │
[Step 4: Classes & Hierarchy] -> Map classes and SubClassOf parent/child relationships
          │
[Step 5 & 6: Properties & Facets] -> Object/Data properties, domain, range, & XML Schema datatypes
          │
[Step 7: Instances & Graph Binding] -> Declare named individuals, owl:imports, & owl:equivalentClass mappings
                                      (Executed by generate_owl_ontology)
```

---

## 🌐 Hybrid Domain Ontology Integration

OwlScribe supports importing established domain ontologies (e.g. SOSA, SSN, SAREF, QUDT, PROV-O) through a two-phase hybrid design:

### 1. Base-First Seed Extraction
When parsing a PDF, pass an existing base ontology file path (`base_ontology_path`) or structured summary (`base_ontology_seed`). The `SeedConceptMatcher` aligns PDF candidate terms with top-level base concepts:
- **Concept Priming**: Primes recognition of synonymous terms (e.g., recognizing "Sensing Unit" in an IEEE spec maps directly to `sosa:Sensor`).
- **Confidence Boosting**: Automatically boosts extraction confidence when candidate terms ground to established domain concepts.
- **Mapping Relation**: Defaults to `owl:equivalentClass` (preserving exact semantic identity), while supporting `rdfs:subClassOf`.
- **Interactive Suggestions**: Recommends candidate base ontologies (`suggested_base_ontologies`) based on normative references and domain keywords.

### 2. Full Graph Binding (Post-Extraction)
Once candidate terms are pulled from the PDF, `generate_owl_ontology` binds the candidate classes into the target ontology graph using `horned-owl`:
- **`imports`**: Emits formal W3C `owl:imports` declarations (`Import(<http://www.w3.org/ns/sosa/>)`).
- **`class_mappings`**: Emits `owl:equivalentClass` or `rdfs:subClassOf` axioms linking local spec classes to base ontology IRIs.
- **Base Graph Merging**: Optionally merges full base OFN graphs into the output file while removing duplicate `OntologyID` headers.

---

## 🛠️ MCP Tools Reference

### 1. `parse_pdf_to_terms`

Extracts raw text, detects specification layout sections, and harvests domain term candidates mapped to McGuinness Steps 1–3, optionally seeding with a base ontology.

#### Input Schema
```json
{
  "pdf_path": "/path/to/specification.pdf",
  "spec_type": "auto", // "iso" | "ieee" | "w3c" | "nist" | "rfc" | "auto"
  "min_confidence": 0.3,
  "base_ontology_path": "/path/to/sosa.ofn", // Optional base ontology file
  "base_ontology_seed": {                   // Optional inline seed object
    "ontology_iri": "http://www.w3.org/ns/sosa/",
    "prefix": "sosa",
    "top_classes": [
      {
        "name": "Sensor",
        "iri": "http://www.w3.org/ns/sosa/Sensor",
        "synonyms": ["Sensing Unit"]
      }
    ]
  }
}
```

#### Output Payload
```json
{
  "step1_domain_scope": {
    "document_title": "IEEE Std 2026-IoT",
    "detected_spec_type": "ieee",
    "domain_scope": "Specification ontology domain extracted from IEEE Std 2026-IoT..."
  },
  "step2_reuse_references": {
    "normative_references": ["ISO/IEC 27000"],
    "candidate_ontologies": [
      "http://www.w3.org/2002/07/owl#",
      "http://www.w3.org/2000/01/rdf-schema#",
      "http://www.w3.org/2004/02/skos/core#"
    ],
    "suggested_base_ontologies": [
      "http://www.w3.org/ns/sosa/"
    ]
  },
  "step3_term_enumeration": {
    "total_terms_found": 2,
    "term_candidates": [
      {
        "term": "Sensing Unit",
        "definition": "Component responsible for capturing physical environment state.",
        "confidence": 0.85,
        "section": "Terms and Definitions",
        "rfc2119_keywords": [],
        "context_snippet": "3.2 Sensing Unit: Component responsible for...",
        "mapped_base_concept": "http://www.w3.org/ns/sosa/Sensor",
        "mapping_relation": "equivalentClass"
      }
    ]
  },
  "detected_sections": ["Scope", "Normative References", "Terms and Definitions"]
}
```

---

### 2. `generate_owl_ontology`

Accepts structured McGuinness input (Steps 4–7), imports base domain graphs, and generates a validated OWL 2 ontology using `horned-owl`.

#### Input Schema
```json
{
  "ontology_iri": "http://example.org/ieee2026#",
  "prefix": "ieee2026",
  "format": "ofn", // "ofn" (OWL Functional Syntax) | "turtle" | "rdfxml"
  "imports": [
    "http://www.w3.org/ns/sosa/"
  ],
  "base_ontology_path": "/path/to/sosa.ofn",
  "classes": [
    {
      "name": "SensingUnit",
      "parent_class": null,
      "comment": "Component capturing physical environment state"
    }
  ],
  "class_mappings": [
    {
      "term": "SensingUnit",
      "target_iri": "http://www.w3.org/ns/sosa/Sensor",
      "mapping_type": "equivalentClass" // "equivalentClass" | "subClassOf"
    }
  ],
  "object_properties": [],
  "data_properties": [],
  "individuals": []
}
```

#### Output Payload
```json
{
  "ontology_iri": "http://example.org/ieee2026#",
  "format": "ofn",
  "class_count": 1,
  "object_property_count": 0,
  "data_property_count": 0,
  "individual_count": 0,
  "axiom_count": 5,
  "serialized_ontology": "Prefix(:=<http://example.org/ieee2026#>)\nOntology(<http://example.org/ieee2026#>\nImport(<http://www.w3.org/ns/sosa/>)\nDeclaration(Class(:SensingUnit))\nEquivalentClasses(:SensingUnit <http://www.w3.org/ns/sosa/Sensor>)\n...)"
}
```

---

## 💡 Industry Use Cases

### 1. ISO Standard Compliance Modeling
- **Problem**: Compliance engineers manually spend hundreds of hours reading multi-page ISO standards (e.g. ISO 27001, ISO 9001, ISO 26262) to create enterprise compliance graph databases.
- **OwlScribe Solution**: Ingests ISO PDFs, harvests Clause 3 definitions, aligns with ISO 27000 base ontologies, generates OWL 2 class hierarchies, and exports functional syntax ontologies for Protégé, Neo4j, or RDF triplestores.

### 2. IEEE Robotics & Power Grid Federation
- **Problem**: IEEE standards (e.g. IEEE 1547 for distributed energy resources, IEEE 1857 for audio/video coding) use domain-specific terms ("Sensing Unit") that need to align with W3C standards like SOSA/SSN.
- **OwlScribe Solution**: Extracts IEEE Clause 3 terms, seeds parsing with SOSA top-level classes, and emits `owl:equivalentClass` mappings into machine-actionable OWL ontologies for smart grid co-simulations (HELICS / OEDISI).

### 3. NIST Cybersecurity Framework Knowledge Graphs
- **Problem**: NIST SP 800-53 security controls contain mandatory compliance terms with strict modal keywords (`MUST`, `SHALL NOT`).
- **OwlScribe Solution**: Captures term definitions and RFC 2119 requirement levels, constructing security ontologies mapped to PROV-O and ODRL for policy reasoning engines.

---

## ⚡ Best Practices & Recommendations

1. **Two-Stage Human-in-the-Loop Refinement**:
   - First call `parse_pdf_to_terms` with `base_ontology_path` or `base_ontology_seed` to harvest raw terms and concept mappings.
   - Review candidate terms with confidence scores below `0.80` and verify suggested base ontologies.
   - Pass refined classes and `class_mappings` to `generate_owl_ontology`.

2. **IRI Prefix Naming Hygiene**:
   - Always supply an absolute, hash-ended base IRI (e.g. `http://standard.org/ns/v1#`).
   - Use PascalCase for class names (`InformationAsset`) and camelCase for properties (`protectsAsset`).

3. **Production Deployment**:
   - Compile with `cargo build --release` for maximum PDF extraction and ontology generation performance.

---

## 📦 Installation & MCP Registration

### Build Binary
```bash
git clone https://github.com/clark-labs-inc/OwlScribe.git
cd OwlScribe
cargo build --release
```

### Register with Antigravity
Add the server entry to your global Antigravity MCP configuration file (`~/.gemini/config/mcp_config.json`):

```json
{
  "mcpServers": {
    "owlscribe": {
      "command": "/home/tslay/dev/OwlScribe/target/release/owlscribe",
      "args": []
    }
  }
}
```

*(Alternatively, in the Antigravity Desktop App / IDE, navigate to **Settings** → **MCP Servers** to configure the binary path graphically).*

---

## 🧪 Developer Quickstart & Testing

Run all unit tests and end-to-end integration tests:
```bash
cargo test
```

Execute a release binary check:
```bash
./target/release/owlscribe
```
*(Communicates over stdio JSON-RPC 2.0)*
