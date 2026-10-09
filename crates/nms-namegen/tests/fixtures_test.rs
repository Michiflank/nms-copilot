//! Checks the port against the upstream `nms_namegen` fixtures and against
//! names read from a real save.

use nms_namegen::{planet_name, region_name, system_name};

fn load(name: &str) -> serde_json::Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn as_u64(v: &serde_json::Value) -> u64 {
    v.as_u64().unwrap_or_else(|| v.as_i64().unwrap() as u64)
}

#[test]
fn system_names_match_fixtures() {
    let data = load("system_names.json");
    let errors: Vec<_> = data
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|rec| {
            let expected = rec[0].as_str().unwrap();
            let code = u64::from_str_radix(rec[2].as_str().unwrap(), 16).unwrap();
            let got = system_name(code, 0);
            (got != expected).then(|| format!("{:012X}: got {got:?}, expected {expected:?}", code))
        })
        .collect();
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

#[test]
fn region_names_match_fixtures() {
    let data = load("region_names.json");
    let errors: Vec<_> = data
        .as_object()
        .unwrap()
        .iter()
        .filter_map(|(grf, expected)| {
            let v = u64::from_str_radix(grf.trim_start_matches("0x"), 16).unwrap();
            let got = region_name(v & 0xFFFF_FFFF, (v >> 32) as u8);
            let expected = expected.as_str().unwrap();
            (got != expected).then(|| format!("{grf}: got {got:?}, expected {expected:?}"))
        })
        .collect();
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

#[test]
fn golden_vectors_match() {
    let data = load("golden_vectors.json");
    let mut errors = Vec::new();
    for rec in data.as_array().unwrap() {
        let code = as_u64(&rec["code"]);
        let galaxy = rec["galaxy"].as_u64().unwrap() as u8;

        let got = region_name(code, galaxy);
        if got != rec["region"].as_str().unwrap() {
            errors.push(format!(
                "region {code:012X}/g{galaxy}: got {got:?}, expected {}",
                rec["region"]
            ));
        }
        let got = system_name(code, galaxy);
        if got != rec["system"].as_str().unwrap() {
            errors.push(format!(
                "system {code:012X}/g{galaxy}: got {got:?}, expected {}",
                rec["system"]
            ));
        }

        let expected = rec["planet"].as_str().unwrap_or("ERR:");
        let seeds: Vec<u64> = rec["seeds"]
            .as_array()
            .unwrap()
            .iter()
            .map(as_u64)
            .collect();
        let planet_id = ((code >> 44) & 0xF) as usize;
        // Upstream indexes seeds[planet_id - 1], which wraps to the last seed
        // for planet 0.
        let seed = if planet_id == 0 {
            seeds.last()
        } else {
            seeds.get(planet_id - 1)
        };
        if let (false, Some(&seed)) = (expected.starts_with("ERR:"), seed) {
            let got = planet_name(seed);
            if got != expected {
                errors.push(format!(
                    "planet {code:012X}/g{galaxy}: got {got:?}, expected {expected:?}"
                ));
            }
        }
    }
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

/// Station names read from a real Euclid save ("<system> Station Tau").
#[test]
fn system_names_match_real_save() {
    assert_eq!(system_name(0x0078_F808_D6D2, 0), "Racking-Bud XIII");
    assert_eq!(system_name(0x00F6_F808_D6D2, 0), "Aghullo");
    assert_eq!(system_name(0x016E_F808_D6D2, 0), "Laurio-Itam XI");
}
