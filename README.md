# 🦉 OwlScribe: Specification PDF to OWL 2 Ontology MCP Server

**OwlScribe** is a high-performance Model Context Protocol (MCP) server written in Rust. It translates technical specification PDFs (ISO, IEEE, W3C, NIST, IETF RFCs) into standardized **OWL 2 ontologies** using native Rust AST primitives (`horned-owl`) and the **Noy & McGuinness 7-Step Ontology Development Methodology**.

---

## 📋 Table of Contents
1. [Architecture & Design Principles](#-architecture--design-principles)
2. [Supported Specification Families](#-supported-specification-families)
3. [The McGuinness 7-Step Workflow](#-the-mcguinness-7-step-workflow)
4. [MCP Tools Reference](#-mcp-tools-reference)
   - [`parse_pdf_to_terms`](#1-parse_pdf_to_terms)
   - [`generate_owl_ontology`](#2-generate_owl_ontology)
5. [Industry Use Cases](#-industry-use-cases)
6. [Best Practices & Recommendations](#-best-practices--recommendations)
7. [Installation & MCP Registration](#-installation--mcp-registration)
8. [Developer Quickstart & Testing](#-developer-quickstart--testing)

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
           |                                                       |
           v                                                       v
+-----------------------+                               +---------------------------+
|   OwlScribe Parser    |                               |   OwlScribe Generator     |
|  - Spec Profiles      |                               |  - McGuinness Steps 4-7   |
|  - Layout Extractor   |                               |  - Horned-Owl AST Builder |
|  - Term Harvester     |                               |  - Serializer (OFN/TTL)   |
+-----------------------+                               +---------------------------+
           |                                                       |
           v                                                       v
  McGuinness Steps 1-3                                  Validated OWL 2 Ontology
 (Domain, Scope, Terms)                                (OFN / Turtle / RDF-XML)
```

### Key Architectural Strengths
- **Type-Safe AST Execution**: Uses `horned-owl` (v2.1) to guarantee that generated ontologies strictly satisfy W3C OWL 2 structural and semantic invariants.
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
[Step 2: Reuse References] --->  Harvest normative references & base ontologies (OWL, RDFS, SKOS, ODRL)
          │
[Step 3: Term Enumeration] --->  Extract term candidates, definitions, & confidence scores
          │                      (Output of parse_pdf_to_terms)
          │
          ▼  <-- User/LLM Confidence Refinement Phase -->
          │
[Step 4: Classes & Hierarchy] -> Map classes and SubClassOf parent/child relationships
          │
[Step 5 & 6: Properties & Facets] -> Object/Data properties, domain, range, & XML Schema datatypes
          │
[Step 7: Instances / Individuals] -> Declare named individuals and ClassAssertion axioms
                                 (Executed by generate_owl_ontology)
```

---

## 🛠️ MCP Tools Reference

### 1. `parse_pdf_to_terms`

Extracts raw text, detects specification layout sections, and harvests domain term candidates mapped to McGuinness Steps 1–3.

#### Input Schema
```json
{
  "pdf_path": "/path/to/specification.pdf",
  "spec_type": "auto", // "iso" | "ieee" | "w3c" | "nist" | "rfc" | "auto"
  "min_confidence": 0.3
}
```

#### Output Payload
```json
{
  "step1_domain_scope": {
    "document_title": "ISO/IEC 27000:2026",
    "detected_spec_type": "iso",
    "domain_scope": "Specification ontology domain extracted from ISO/IEC 27000..."
  },
  "step2_reuse_references": {
    "normative_references": ["ISO/IEC 27001", "ISO/IEC 27002"],
    "candidate_ontologies": [
      "http://www.w3.org/2002/07/owl#",
      "http://www.w3.org/2000/01/rdf-schema#",
      "http://www.w3.org/2004/02/skos/core#"
    ]
  },
  "step3_term_enumeration": {
    "total_terms_found": 12,
    "term_candidates": [
      {
        "term": "Confidentiality",
        "definition": "Property that information is not made available or disclosed to unauthorized entities.",
        "confidence": 0.95,
        "section": "Terms and Definitions",
        "rfc2119_keywords": ["SHALL NOT"],
        "context_snippet": "3.1 Confidentiality: Property that information is not made available..."
      }
    ]
  },
  "detected_sections": ["Scope", "Normative References", "Terms and Definitions"]
}
```

---

### 2. `generate_owl_ontology`

Accepts structured McGuinness input (Steps 4–7) and generates a validated OWL 2 ontology using `horned-owl`.

#### Input Schema
```json
{
  "ontology_iri": "http://iso.org/ontology/27000#",
  "prefix": "iso27000",
  "format": "ofn", // "ofn" (OWL Functional Syntax) | "turtle" | "rdfxml"
  "classes": [
    {
      "name": "SecurityProperty",
      "parent_class": null,
      "comment": "Top-level security attribute class"
    },
    {
      "name": "Confidentiality",
      "parent_class": "SecurityProperty",
      "comment": "Property of information secrecy"
    }
  ],
  "object_properties": [
    {
      "name": "protectsAsset",
      "domain": "SecurityProperty",
      "range": "InformationAsset"
    }
  ],
  "data_properties": [
    {
      "name": "hasClassificationLevel",
      "domain": "InformationAsset",
      "range": "xsd:integer"
    }
  ],
  "individuals": [
    {
      "name": "ClassifiedDocument_A",
      "class_name": "InformationAsset"
    }
  ]
}
```

#### Output Payload
```json
{
  "ontology_iri": "http://iso.org/ontology/27000#",
  "format": "ofn",
  "class_count": 2,
  "object_property_count": 1,
  "data_property_count": 1,
  "individual_count": 1,
  "axiom_count": 8,
  "serialized_ontology": "Prefix(:=<http://iso.org/ontology/27000#>)\nOntology(<http://iso.org/ontology/27000#>\nDeclaration(Class(:SecurityProperty))\nDeclaration(Class(:Confidentiality))\nSubClassOf(:Confidentiality :SecurityProperty)\n...)"
}
```

---

## 💡 Industry Use Cases

### 1. ISO Standard Compliance Modeling
- **Problem**: Compliance engineers manually spend hundreds of hours reading multi-page ISO standards (e.g. ISO 27001, ISO 9001, ISO 26262) to create enterprise compliance graph databases.
- **OwlScribe Solution**: Ingests ISO PDFs, harvests Clause 3 definitions, generates OWL 2 class hierarchies, and exports functional syntax ontologies to feed Protégé, Neo4j, or RDF triplestores.

### 2. IEEE Robotics & Power Grid Federation
- **Problem**: IEEE standards (e.g. IEEE 1547 for distributed energy resources, IEEE 1857 for audio/video coding) have dense mathematical and structural definitions.
- **OwlScribe Solution**: Extracts IEEE Clause 3 terms, categorizes sub-domain classes, and builds machine-actionable OWL ontologies for smart grid co-simulations (HELICS / OEDISI).

### 3. NIST Cybersecurity Framework Knowledge Graphs
- **Problem**: NIST SP 800-53 security controls contain mandatory compliance terms with strict modal keywords (`MUST`, `SHALL NOT`).
- **OwlScribe Solution**: Captures term definitions and RFC 2119 requirement levels, constructing security ontologies for automated policy reasoning engines.

---

## ⚡ Best Practices & Recommendations

1. **Two-Stage Human-in-the-Loop Refinement**:
   - First call `parse_pdf_to_terms` to harvest raw terms.
   - Review term candidates with confidence scores below `0.80`.
   - Send approved class trees to `generate_owl_ontology`.

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
