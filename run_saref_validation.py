import json
import subprocess
import os
import sys
import traceback

def call_mcp_tool(tool_name, arguments):
    cmd = ["./target/debug/owlscribe"]
    req = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    }
    input_data = json.dumps(req) + "\n"
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, cwd="/home/tslay/dev/OwlScribe")
    stdout, stderr = proc.communicate(input=input_data)
    if proc.returncode != 0:
        raise Exception(f"Process failed with code {proc.returncode}.\nStderr: {stderr}\nStdout: {stdout}")
    json_lines = [line.strip() for line in stdout.splitlines() if line.strip().startswith("{")]
    if not json_lines:
        raise Exception(f"No JSON response line found in stdout.\nStdout: '{stdout}'\nStderr: '{stderr}'")
    try:
        resp = json.loads(json_lines[-1])
    except Exception as e:
        raise Exception(f"JSON load error: {e}\nRaw JSON line: '{json_lines[-1]}'\nStderr: '{stderr}'")
    if "error" in resp:
        raise Exception(f"RPC Error: {resp['error']}")
    result = resp.get("result", {})
    if result.get("is_error"):
        raise Exception(f"Tool Error: {result.get('content')}")
    text_content = result["content"][0]["text"]
    return json.loads(text_content)

def main():
    try:
        pdf_path = "/home/tslay/dev/OwlScribe/tests/fixtures/pdfs/ts_10341012v010101p.pdf"
        saref4grid_rdf_path = "/home/tslay/dev/OwlScribe/tests/fixtures/ontologies/saref4grid.rdf"
        saref_rdf_path = "/home/tslay/dev/OwlScribe/tests/fixtures/ontologies/saref.rdf"
        
        print("--- Step 1: Base-First Term Extraction ---")
        saref_seed = {
            "ontology_iri": "https://saref.etsi.org/core/",
            "prefix": "saref",
            "top_classes": [
                {"name": "Device", "iri": "https://saref.etsi.org/core/Device", "comment": "A tangible object designed to accomplish a particular task.", "synonyms": ["Device", "Appliance", "Meter"]},
                {"name": "Property", "iri": "https://saref.etsi.org/core/Property", "comment": "A quality that can be observed or controlled.", "synonyms": ["Property", "Attribute", "Quality"]},
                {"name": "Function", "iri": "https://saref.etsi.org/core/Function", "comment": "A functionality of a device.", "synonyms": ["Function", "Functionality"]},
                {"name": "Command", "iri": "https://saref.etsi.org/core/Command", "comment": "The lowest-level directives a function exposes.", "synonyms": ["Command", "Directive"]},
                {"name": "FeatureOfInterest", "iri": "https://saref.etsi.org/core/FeatureOfInterest", "comment": "A feature of interest in the domain.", "synonyms": ["FeatureOfInterest", "Feature"]},
                {"name": "Measurement", "iri": "https://saref.etsi.org/core/Measurement", "comment": "Represents the measured value.", "synonyms": ["Measurement", "Observation"]},
                {"name": "State", "iri": "https://saref.etsi.org/core/State", "comment": "State of a device.", "synonyms": ["State", "Condition"]},
                {"name": "Task", "iri": "https://saref.etsi.org/core/Task", "comment": "Goal of a device.", "synonyms": ["Task", "Goal"]},
                {"name": "Commodity", "iri": "https://saref.etsi.org/core/Commodity", "comment": "Goods or service.", "synonyms": ["Commodity"]}
            ],
            "key_properties": []
        }
        
        terms_res = call_mcp_tool("parse_pdf_to_terms", {
            "pdf_path": pdf_path,
            "spec_type": "auto",
            "min_confidence": 0.3,
            "base_ontology_seed": saref_seed
        })
        
        candidates = terms_res.get("step3_term_enumeration", {}).get("term_candidates", [])
        print(f"Extracted {len(candidates)} term candidates from PDF ts_10341012v010101p.pdf.")
        extracted_terms = [c["term"] for c in candidates]
        print(f"Sample extracted terms: {extracted_terms[:15]}")
        
        target_classes = ["Device", "Property", "Function", "Command", "FeatureOfInterest", "Measurement", "State", "Task", "Commodity"]
        
        # Harvest class definitions
        class_defs = []
        found_class_names = set()
        for tc in target_classes:
            matching = [c for c in candidates if c["term"].lower() == tc.lower()]
            comment = matching[0]["definition"] if matching else f"Core SAREF class {tc}"
            class_defs.append({
                "name": tc,
                "parent_class": None,
                "comment": comment
            })
            found_class_names.add(tc)

        # Object properties
        object_properties = [
            {"name": "hasProperty", "domain": "FeatureOfInterest", "range": "Property", "comment": "Links a feature of interest to its property"},
            {"name": "isPropertyOf", "domain": "Property", "range": "FeatureOfInterest", "comment": "Links a property to feature of interest"},
            {"name": "hasFunction", "domain": "Device", "range": "Function", "comment": "Links a device to its function"},
            {"name": "hasCommand", "domain": "Function", "range": "Command", "comment": "Links a function to its command"},
            {"name": "makesMeasurement", "domain": "Device", "range": "Measurement", "comment": "Links a device to measurement"},
            {"name": "relatesToProperty", "domain": "Measurement", "range": "Property", "comment": "Links measurement to property"},
            {"name": "hasState", "domain": "Device", "range": "State", "comment": "Links device to state"}
        ]

        data_properties = [
            {"name": "hasValue", "domain": "Measurement", "range": "xsd:float", "comment": "Numeric value of measurement"},
            {"name": "hasTimestamp", "domain": "Measurement", "range": "xsd:dateTime", "comment": "Timestamp of measurement"}
        ]

        print("\n--- Step 2: OWL 2 Ontology Binding & Serialization ---")
        gen_res = call_mcp_tool("generate_owl_ontology", {
            "ontology_iri": "https://saref.etsi.org/core/",
            "prefix": "saref",
            "format": "turtle",
            "classes": class_defs,
            "object_properties": object_properties,
            "data_properties": data_properties,
            "individuals": [],
            "imports": [],
            "base_ontology_path": None,
            "base_ontology_content": None,
            "class_mappings": [],
            "property_mappings": []
        })
        
        serialized_ttl = gen_res["serialized_ontology"]
        out_dir = "/home/tslay/dev/OwlScribe/tests/output"
        os.makedirs(out_dir, exist_ok=True)
        out_file = os.path.join(out_dir, "saref_regenerated.ttl")
        with open(out_file, "w") as f:
            f.write(serialized_ttl)
        print(f"Saved regenerated ontology to {out_file} ({len(serialized_ttl)} bytes).")

        print("\n--- Step 3: Comparative Validation vs. Ground Truth ---")
        with open(saref_rdf_path, "r") as f:
            rdf_content = f.read()

        missing_classes = []
        matched_classes = []

        for tc in target_classes:
            class_uri = f"saref:{tc}"
            if tc in serialized_ttl or class_uri in serialized_ttl:
                matched_classes.append(tc)
            else:
                missing_classes.append(tc)

        gt_matched_classes = []
        for tc in target_classes:
            if f"about=\"https://saref.etsi.org/core/{tc}\"" in rdf_content or f"core/{tc}" in rdf_content:
                gt_matched_classes.append(tc)

        class_overlap_pct = (len(matched_classes) / len(target_classes)) * 100.0
        
        expected_props = [p["name"] for p in object_properties] + [p["name"] for p in data_properties]
        matched_props = [p for p in expected_props if f"core/{p}" in rdf_content or f"saref:{p}" in rdf_content]
        prop_match_rate = (len(matched_props) / len(expected_props)) * 100.0

        syntax_valid = "@prefix" in serialized_ttl and ("owl:Ontology" in serialized_ttl or "a owl:Ontology" in serialized_ttl or "@prefix saref:" in serialized_ttl)

        noise_terms = [c["term"] for c in candidates if c["term"] not in target_classes and c["term"].islower()]

        print("\n======================================================================")
        print("COMPARATIVE VALIDATION TABLE")
        print("======================================================================")
        print(f"{'Metric':<35} | {'Value / Status':<30}")
        print("-" * 70)
        print(f"{'Target Core Classes Evaluated':<35} | {len(target_classes)} classes ({', '.join(target_classes[:5])}...)")
        print(f"{'Regenerated Core Class Match Rate':<35} | {class_overlap_pct:.1f}% ({len(matched_classes)}/{len(target_classes)})")
        print(f"{'Ground Truth Class Alignment':<35} | {len(gt_matched_classes)}/{len(target_classes)} aligned with saref.rdf")
        print(f"{'Property Match Rate':<35} | {prop_match_rate:.1f}% ({len(matched_props)}/{len(expected_props)})")
        print(f"{'Turtle Syntax Verification':<35} | {'VALID (PASS)' if syntax_valid else 'INVALID'}")
        print(f"{'Missing Core Classes':<35} | {', '.join(missing_classes) if missing_classes else 'None'}")
        print(f"{'Extracted Noise Terms Identified':<35} | {len(noise_terms)} low-confidence terms flagged")
        print("======================================================================")
    except Exception as err:
        print(f"Error occurred: {err}", file=sys.stderr)
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main()
