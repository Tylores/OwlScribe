# 🦉 OwlScribe: Specification PDF to OWL 2 Ontology MCP Server

**OwlScribe** is a high-performance Model Context Protocol (MCP) server written in Rust. It translates technical specification PDFs (ISO, IEEE, W3C, NIST, IETF RFCs) into standardized **OWL 2 ontologies** using native Rust AST primitives (`horned-owl`) and the **Noy & McGuinness 7-Step Ontology Development Methodology**.

Featuring an **Agentic 4-Phase Discovery Workflow**: section-by-section table of contents mapping, targeted section reading, intermediate term staging, and final W3C graph binding (`owl:imports`, `owl:equivalentClass`, `rdfs:subClassOf`).

---

## 📋 Table of Contents
1. [Architecture & Design Principles](#-architecture--design-principles)
2. [Supported Specification Families](#-supported-specification-families)
3. [The McGuinness 7-Step Agentic Workflow](#-the-mcguinness-7-step-agentic-workflow)
4. [Hybrid Domain Ontology Integration](#-hybrid-domain-ontology-integration)
5. [MCP Tools Reference](#-mcp-tools-reference)
   - [`get_pdf_sections` / `get_pdf_toc`](#1-get_pdf_sections--get_pdf_toc)
   - [`read_pdf_section`](#2-read_pdf_section)
   - [`propose_ontology_terms`](#3-propose_ontology_terms)
   - [`generate_owl_ontology`](#4-generate_owl_ontology)
6. [Interactive Agent System Prompt](#-interactive-agent-system-prompt)
7. [Industry Use Cases](#-industry-use-cases)
8. [Installation & MCP Registration](#-installation--mcp-registration)
9. [Developer Quickstart & Testing](#-developer-quickstart--testing)

---

## 🏗️ Architecture & Design Principles

OwlScribe is architected as an asynchronous, zero-cost-abstraction Rust service using standard I/O (stdio) JSON-RPC 2.0 transport following the Model Context Protocol specification.

```
+-----------------------------------------------------------------------------------+
|                                  MCP Client (LLM / Agent)                         |
+-----------------------------------------------------------------------------------+
       |                    ^                   ^                    ^
  (1) get_pdf_sections  (2) read_pdf_section  (3) propose_terms  (4) generate_owl
       |                    |                   |                    |
       v                    v                   v                    v
+---------------+    +---------------+   +-------------------+  +-------------------+
| Section Map   |    | Section Text  |   | Staging Memory    |  | Horned-OWL        |
| - TOC Parser  |    | - Target Read |   | - Term Stager     |  | - Steps 4-7 AST   |
| - Section IDs |    | - Candidate   |   | - Class/Prop      |  | - Base Merger     |
| - Page Ranges |    |   Extractor   |   |   Inventory       |  | - Serializer      |
+---------------+    +---------------+   +-------------------+  +-------------------+
```

### Key Architectural Strengths
- **Type-Safe AST Execution**: Uses `horned-owl` (v2.1) to guarantee that generated ontologies satisfy W3C OWL 2 structural and semantic invariants.
- **Agentic 4-Phase Discovery Workflow**: Eliminates single-pass raw text dumps in favor of selective section inspection, human/agent clarification, intermediate term staging, and graph serialization.
- **Persistent Session Staging Store**: In-memory and file-backed staging memory (`STAGED_INVENTORY`) preserving staged terms across tool calls.
- **Hybrid Base Ontology Seeding**: Ingests base domain ontologies (Turtle `.ttl`, JSON-LD `.jsonld`, OWL Functional Syntax `.ofn`, RDF/XML `.rdf`) to ground PDF terminology.
- **Multi-Format Serialization**: Emits Turtle (`.ttl` - default), JSON-LD (`.jsonld`), OWL Functional Syntax (`.ofn`), and RDF/XML (`.owl`).

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

## 🧩 The McGuinness 7-Step Agentic Workflow

```
[Phase 1: TOC & Section Selection] ---> Call get_pdf_sections to map document sections
          │
[Phase 2: Section Reading] ----------> Call read_pdf_section for target standard clauses
          │                            Clarify ambiguous terms & superclasses with user
          │
[Phase 3: Intermediate Staging] -----> Call propose_ontology_terms to accumulate section inventory
          │
[Phase 4: Horned-OWL Serialization] -> Call generate_owl_ontology to emit W3C OWL 2 (.ttl)
```

---

## 🛠️ MCP Tools Reference

### 1. `get_pdf_sections` / `get_pdf_toc`
Returns section titles, section IDs (`sec_1`, `sec_2`), page ranges, and preview snippets from specification PDFs.

### 2. `read_pdf_section`
Retrieves targeted section text and section candidate terms by `section_id`, `section_title`, or page ranges (`page_start`/`page_end`).

### 3. `propose_ontology_terms`
Agent-facing tool to stage, classify (`owl:Class`, `owl:ObjectProperty`, `owl:DatatypeProperty`), and validate candidate terms and base ontology mappings section-by-section.

### 4. `generate_owl_ontology`
Incorporates all section terms staged via `propose_ontology_terms`, imports base graphs, and generates the final W3C OWL 2 ontology.

---

## 🤖 Interactive Agent System Prompt

```text
You are an expert Ontology Engineer building a W3C OWL 2.0 ontology from a standard specification PDF.

CRITICAL INSTRUCTION: Do NOT attempt to extract all ontology terms in a single pass. You MUST execute an interactive, multi-step discovery process.

### PHASE 1: Table of Contents & Section Selection
1. Call get_pdf_sections to map out the PDF standard.
2. Identify sections containing normative definitions, domain models, or structural hierarchies (e.g., "3. Terms and Definitions", "5. System Architecture").
3. Report your chosen sections to the user before reading them.

### PHASE 2: Section-by-Section Term Extraction & Clarification
For each target section:
1. Fetch the section text using read_pdf_section.
2. Extract candidate terms and classify each candidate into:
   - owl:Class (Entities/Types)
   - owl:ObjectProperty (Relationships between entities)
   - owl:DatatypeProperty (Attributes/Values)
3. STOP AND CLARIFY: If a term is ambiguous, has multiple meanings, or conflicts with the base/seed ontology (if loaded), STOP and ask the user:
   - "Term 'X' could be modeled as an ObjectProperty or a Class. Given context Y, how should we represent it?"
   - "Term 'Z' overlaps with base ontology class `sosa:Sensor`. Should we create a subclass or use `owl:equivalentClass`?"

### PHASE 3: Intermediate Staging & Verification
1. Call propose_ontology_terms with your validated set for the section.
2. Present a brief summary table of the staged terms to the user.
3. Proceed to the next section only after staging is verified.

### PHASE 4: Horned-OWL Serialization
Once all relevant sections are reviewed, call generate_owl_ontology to build and validate the final .ttl output using Horned-OWL.
```

---

## 💡 Industry Use Cases

- **Smart Grid Co-Simulation**: Modeling IEEE 1547 and ETSI SAREF4GRID standards for HELICS / OEDISI smart grid federations.
- **ISO Standard Compliance Modeling**: Translating ISO 27001, ISO 9001, and ISO 26262 into compliance graph databases.
- **NIST Security Ontologies**: Structuring NIST SP 800-53 security control requirements and RFC 2119 keywords.

---

## 📦 Installation & MCP Registration

### Build Binary
```bash
git clone https://github.com/clark-labs-inc/OwlScribe.git
cd OwlScribe
cargo build --release
```

### Register with Antigravity
Add the server entry to `~/.gemini/config/mcp_config.json`:
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

---

## 🧪 Developer Quickstart & Testing

Run unit and integration test suite:
```bash
cargo test
```

Run SAREF comparative validation script:
```bash
python3 run_saref_validation.py
```
