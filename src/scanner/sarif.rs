use crate::scanner::rules::Finding;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Build a valid SARIF 2.1.0 JSON document compatible with GitHub Code Scanning.
pub fn to_sarif_json(findings: &[Finding]) -> Result<String> {
    let mut rules_map: BTreeMap<String, Value> = BTreeMap::new();
    let mut results_vec: Vec<Value> = Vec::with_capacity(findings.len());

    for f in findings {
        rules_map.entry(f.rule_id.clone()).or_insert_with(|| {
            json!({
                "id": f.rule_id,
                "name": f.category.replace(' ', ""),
                "shortDescription": {
                    "text": f.message
                },
                "fullDescription": {
                    "text": format!("{} Remediation: {}", f.message, f.remediation)
                },
                "help": {
                    "text": f.remediation,
                    "markdown": format!("**Remediation**: {}", f.remediation)
                },
                "defaultConfiguration": {
                    "level": f.severity.sarif_level()
                },
                "properties": {
                    "category": f.category,
                    "security-severity": match f.severity {
                        crate::cli::Severity::Critical => "9.5",
                        crate::cli::Severity::High => "7.8",
                        crate::cli::Severity::Medium => "5.5",
                        crate::cli::Severity::Low => "3.0",
                    }
                }
            })
        });

        let normalized_uri = f.file.replace('\\', "/");
        results_vec.push(json!({
            "ruleId": f.rule_id,
            "level": f.severity.sarif_level(),
            "message": {
                "text": format!("{} (Remediation: {})", f.message, f.remediation)
            },
            "locations": [
                {
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": normalized_uri,
                            "uriBaseId": "%SRCROOT%"
                        },
                        "region": {
                            "startLine": f.line.max(1),
                            "snippet": {
                                "text": f.snippet
                            }
                        }
                    }
                }
            ]
        }));
    }

    let sarif_doc = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "ops-ecc",
                        "informationUri": "https://github.com/bilal-talha-dev/ops-ecc",
                        "version": env!("CARGO_PKG_VERSION"),
                        "rules": rules_map.into_values().collect::<Vec<Value>>()
                    }
                },
                "results": results_vec
            }
        ]
    });

    Ok(serde_json::to_string_pretty(&sarif_doc)?)
}
