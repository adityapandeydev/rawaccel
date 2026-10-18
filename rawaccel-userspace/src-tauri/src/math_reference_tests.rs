//! Shared C++ oracle samples. These tests never open the driver.
use super::*;
use crate::config::AppAccelArgs;

#[test]
fn all_modes_match_shared_cpp_reference() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/math/reference.json")).unwrap();
    let absolute = reference["absolute_tolerance"].as_f64().unwrap();
    let relative = reference["relative_tolerance"].as_f64().unwrap();
    let mut failures = Vec::new();
    let mut maximum_absolute_error = 0.0_f64;
    let mut maximum_relative_error = 0.0_f64;
    assert!(!reference["cases"].as_array().unwrap().is_empty());
    for case in reference["cases"].as_array().unwrap() {
        let app: AppAccelArgs = serde_json::from_value(case["args"].clone()).unwrap();
        let args = app
            .to_native()
            .unwrap_or_else(|error| panic!("{}: {error}", case["name"]));
        let mut prepared = args;
        init_accel_union(&mut accel_union::default(), &mut prepared);
        assert_eq!(
            case["xs"].as_array().unwrap().len(),
            case["expected"].as_array().unwrap().len()
        );
        let table_size = if args.mode == accel_mode::synchronous && args.gain {
            97
        } else {
            0
        };
        assert_eq!(case["table_bits"].as_array().unwrap().len(), table_size);
        for (i, bits) in case["table_bits"].as_array().unwrap().iter().enumerate() {
            assert_eq!(
                prepared.data[i].to_bits() as u64,
                bits.as_u64().unwrap(),
                "{}: packed sample {i}",
                case["name"]
            );
        }
        for (x, expected) in case["xs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["expected"].as_array().unwrap())
            .rev()
        {
            let x = x.as_f64().unwrap();
            let actual = evaluate_curve(&args, x);
            let matches = if let Some(expected) = expected.as_f64() {
                let error = (actual - expected).abs();
                maximum_absolute_error = maximum_absolute_error.max(error);
                if expected != 0.0 {
                    maximum_relative_error = maximum_relative_error.max(error / expected.abs());
                }
                actual.is_finite()
                    && (actual - expected).abs() <= absolute + relative * expected.abs()
            } else {
                match expected.as_str().unwrap() {
                    "NaN" => actual.is_nan(),
                    "Infinity" => actual == f64::INFINITY,
                    "-Infinity" => actual == f64::NEG_INFINITY,
                    value => panic!("Unknown oracle value {value}"),
                }
            };
            if !matches {
                failures.push(format!(
                    "{} at {x}: Rust {actual}, C++ {expected}",
                    case["name"]
                ));
            }
        }
    }
    eprintln!("C++ comparison: max absolute error {maximum_absolute_error:e}, max relative error {maximum_relative_error:e}");
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
