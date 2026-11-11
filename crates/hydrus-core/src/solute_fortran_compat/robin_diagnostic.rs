//! Diagnostic-only, ignored Robin coefficient/error audit.
//!
//! This module is compiled only for crate unit tests and is run explicitly with
//! `cargo test -p hydrus-core robin_coefficient_error_audit -- --ignored`.
//! It does not participate in production assembly, producer tools, signed
//! bounds, gates, or the existing `.development/23-final` evidence.

#![cfg(test)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};

use super::assembly::{assemble_interior_into, AssemblyInputs, AssemblyLevel};
use super::transport::{diagnostic, pecour, PecletInputs};
use crate::{
    hydraulics::Mesh1D,
    solute::{
        advance_solute_step_with_processes, AcceptedWaterSnapshot, EquilibriumSorption,
        SoluteBackend, SoluteBoundaries, SoluteBoundaryMode as Boundary, SoluteConfig,
        SoluteProcessInput, SoluteSpecies, SoluteState, SoluteTransportInput, SoluteUnits,
        SoluteWorkspace,
    },
};

const LENGTH: f64 = 2.0;
const Q: f64 = 0.43;
const THETA: f64 = 0.43;
const RHO: f64 = 0.8;
const DISPERSION: f64 = 1.0;
const KD: f64 = 0.26875;
const RATE: f64 = 0.2;
const INLET: f64 = 1.0;
const ANALYSIS: [f64; 3] = [0.25, 0.5, 1.0];

type MatrixRows = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

const CONFIGS: [(usize, f64); 6] = [
    (129, 1.0 / 128.0),
    (193, 1.0 / 128.0),
    (257, 1.0 / 128.0),
    (129, 1.0 / 256.0),
    (193, 1.0 / 256.0),
    (257, 1.0 / 256.0),
];

const REFERENCE_IDS: [(usize, &str); 3] = [
    (
        129,
        "ebed7fd885832a91bfe6c7135279fe50cf058d0dba91e9b11803ff9fe9f9b1a9",
    ),
    (
        193,
        "ed4f6655c34782300173d21cd6e60ca523eea248ea236a05368dc5abaa23135b",
    ),
    (
        257,
        "cbcc4b38cb5d84f3200dc1b0446018cda75079b0caadf36dd7cd7c7705d44acf",
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn output_root() -> PathBuf {
    repo_root().join(".development/23-robin-audit")
}

fn coordinates(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 * LENGTH / (n - 1) as f64).collect()
}

/// `scalar_robin` `initial.kind = steady_plus_mode` from CASES_V2.json.
fn initial_concentration(coordinates: &[f64]) -> Vec<f64> {
    let root_mode = 0.6532711870944032_f64;
    let velocity = Q / THETA;
    let retardation = 1.0 + RHO * KD / THETA;
    let z = retardation * RATE;
    let root = (velocity * velocity + 4.0 * DISPERSION * z).sqrt();
    let plus = (velocity + root) / (2.0 * DISPERSION);
    let minus = -2.0 * z / (velocity + root);
    let ratio = minus / plus;
    let amplitude =
        velocity / (DISPERSION * plus * (1.0 - ratio * ratio * ((minus - plus) * LENGTH).exp()));
    coordinates
        .iter()
        .map(|s| {
            amplitude * ((minus * s).exp() - ratio * (minus * LENGTH + plus * (s - LENGTH)).exp())
                + 0.1
                    * (velocity / (2.0 * DISPERSION) * (s - LENGTH / 2.0)).exp()
                    * (root_mode * (s - LENGTH / 2.0)).cos()
        })
        .collect()
}

fn mesh_weights(coordinates: &[f64]) -> Vec<f64> {
    let mut weights = vec![0.0; coordinates.len()];
    weights[0] = (coordinates[1] - coordinates[0]) / 2.0;
    for i in 1..coordinates.len() - 1 {
        weights[i] = (coordinates[i + 1] - coordinates[i - 1]) / 2.0;
    }
    let last = coordinates.len() - 1;
    weights[last] = (coordinates[last] - coordinates[last - 1]) / 2.0;
    weights
}

fn load_reference(n: usize) -> Result<Value, String> {
    let id = REFERENCE_IDS
        .iter()
        .find(|(nodes, _)| *nodes == n)
        .map(|(_, id)| *id)
        .ok_or_else(|| format!("no reference identity for {n} nodes"))?;
    let path = repo_root()
        .join(".development/23-final/reference-cache")
        .join(id)
        .join("reference.json");
    let text = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

fn row_at<'a>(reference: &'a Value, group: &str, time: f64) -> Result<&'a Value, String> {
    reference["reference"][group]
        .as_array()
        .ok_or_else(|| format!("reference {group} missing"))?
        .iter()
        .find(|row| {
            row["time"]
                .as_f64()
                .is_some_and(|value| (value - time).abs() < 1.0e-12)
        })
        .ok_or_else(|| format!("reference {group} time {time} missing"))
}

fn number(value: &Value, label: &str) -> Result<f64, String> {
    value
        .as_f64()
        .ok_or_else(|| format!("missing numeric {label}"))
}

struct Profile {
    time: f64,
    concentration: Vec<f64>,
    cumulative_outflow: f64,
}

struct Run {
    n: usize,
    profiles: Vec<Profile>,
    weight_calls: usize,
}

fn simulate(n: usize, dt: f64, force_upstream: bool, stable: bool) -> Result<Run, String> {
    diagnostic::set_force_upstream_weight(force_upstream);
    diagnostic::set_stable_weights(stable);
    diagnostic::reset_weight_calls();
    let result = (|| {
        let coordinates = coordinates(n);
        let mesh = Mesh1D::new(coordinates.clone(), vec![0; n], None, 1, 0, 0.0)
            .map_err(|e| e.to_string())?;
        let species = vec![SoluteSpecies {
            name: "S1".into(),
            decay_rate: RATE,
            equilibrium_sorption: EquilibriumSorption::Linear {
                distribution_coefficient: KD,
            },
            kinetic_sorption: None,
        }];
        let boundaries = vec![SoluteBoundaries {
            lower: Boundary::TotalFlux { flux: Q * INLET },
            upper: Boundary::AdvectiveOutflow,
        }];
        let config = SoluteConfig::new_with_species_boundaries(
            SoluteUnits {
                concentration: "C_ref".into(),
                sorbed_mass: "C_ref cm3/g".into(),
            },
            species,
            boundaries,
            1,
            &[],
        )
        .map_err(|e| e.to_string())?
        .with_backend(SoluteBackend::FortranCompatible);
        let initial = initial_concentration(&coordinates);
        let mut state = SoluteState::new(&config, &mesh, 0.0, initial.clone(), vec![0.0; n])
            .map_err(|e| e.to_string())?;
        let mut workspace = SoluteWorkspace::new(n, 1).map_err(|e| e.to_string())?;
        let theta = vec![THETA; n];
        let flux = vec![Q; n + 1];
        let dispersion = vec![DISPERSION; n - 1];
        let density = vec![RHO; n];
        let steps = (1.0 / dt).round() as usize;
        let mut cumulative_outflow = 0.0;
        let mut previous_outlet = -Q * initial[n - 1];
        let mut profiles = Vec::new();
        for step in 1..=steps {
            let accepted_time = step as f64 * dt;
            advance_solute_step_with_processes(
                &mut state,
                &mut workspace,
                &mesh,
                &config,
                SoluteTransportInput {
                    accepted_water: AcceptedWaterSnapshot {
                        accepted_time,
                        water_contents: &theta,
                        face_fluxes: &flux,
                    },
                    start_water_contents: &theta,
                    face_dispersion: &dispersion,
                    timestep: dt,
                    minimum_pivot: 1e-12,
                },
                Some(SoluteProcessInput {
                    bulk_density: &density,
                    max_iterations: 20,
                    nonlinear_tolerance: 1e-6,
                }),
            )
            .map_err(|e| format!("step {step}: {e}"))?;
            let concentration = state.concentrations()[..n].to_vec();
            let outlet = -Q * concentration[n - 1];
            cumulative_outflow += dt * (previous_outlet + outlet) / 2.0;
            previous_outlet = outlet;
            if ANALYSIS
                .iter()
                .any(|time| (accepted_time - *time).abs() < 1.0e-12)
            {
                profiles.push(Profile {
                    time: accepted_time,
                    concentration,
                    cumulative_outflow,
                });
            }
        }
        let weight_calls = diagnostic::weight_calls();
        Ok(Run {
            n,
            profiles,
            weight_calls,
        })
    })();
    diagnostic::set_stable_weights(false);
    diagnostic::set_force_upstream_weight(false);
    result
}

fn inventory(concentration: &[f64], weights: &[f64]) -> (f64, f64) {
    let dissolved: f64 = weights
        .iter()
        .zip(concentration)
        .map(|(w, c)| w * THETA * c)
        .sum();
    let sorbed = RHO
        * KD
        * weights
            .iter()
            .zip(concentration)
            .map(|(w, c)| w * c)
            .sum::<f64>();
    (dissolved, dissolved + sorbed)
}

fn error_summary(run: &Run, reference: &Value) -> Result<Vec<Value>, String> {
    let coordinates = coordinates(run.n);
    let weights = mesh_weights(&coordinates);
    let total_weight: f64 = weights.iter().sum();
    let mut rows = Vec::new();
    for profile in &run.profiles {
        let ref_profile = row_at(reference, "profiles", profile.time)?;
        let expected: Vec<f64> = ref_profile["concentration"][0]
            .as_array()
            .ok_or("missing reference concentration")?
            .iter()
            .map(|value| number(value, "reference concentration"))
            .collect::<Result<_, _>>()?;
        if expected.len() != run.n {
            return Err(format!("reference length {} != {}", expected.len(), run.n));
        }
        let differences: Vec<f64> = profile
            .concentration
            .iter()
            .zip(&expected)
            .map(|(actual, truth)| actual - truth)
            .collect();
        let l1 = differences
            .iter()
            .zip(&weights)
            .map(|(diff, weight)| weight * diff.abs())
            .sum::<f64>()
            / total_weight;
        let l2 = (differences
            .iter()
            .zip(&weights)
            .map(|(diff, weight)| weight * diff * diff)
            .sum::<f64>()
            / total_weight)
            .sqrt();
        let linf = differences.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
        let interior_linf = differences[1..differences.len() - 1]
            .iter()
            .fold(0.0_f64, |m, v| m.max(v.abs()));
        let endpoint_signed = [differences[0], differences[differences.len() - 1]];
        let (dissolved, total) = inventory(&profile.concentration, &weights);
        let continuum = row_at(reference, "continuum", profile.time)?;
        let ref_dissolved = number(&continuum["dissolved"][0], "reference dissolved")?;
        let ref_total = number(&continuum["total"][0], "reference total")?;
        let ref_boundary_outflow = number(
            &continuum["boundary_cumulative"][0][1],
            "reference boundary cumulative",
        )?;
        let ref_instantaneous_outflow =
            number(&continuum["endpoint_flux"][0][1], "reference endpoint flux")?;
        let instantaneous_outflow = -Q * profile.concentration[run.n - 1];
        rows.push(json!({
            "time": profile.time,
            "concentration": {
                "linf": linf,
                "l1": l1,
                "l2": l2,
                "endpoint_linf": endpoint_signed[0].abs().max(endpoint_signed[1].abs()),
                "interior_linf": interior_linf,
                "signed_inlet": endpoint_signed[0],
                "signed_outlet": endpoint_signed[1],
            },
            "inventory": {
                "dissolved_actual": dissolved,
                "dissolved_reference": ref_dissolved,
                "dissolved_signed_error": dissolved - ref_dissolved,
                "total_actual": total,
                "total_reference": ref_total,
                "total_signed_error": total - ref_total,
            },
            "outflow": {
                "instantaneous_actual": instantaneous_outflow,
                "instantaneous_reference": ref_instantaneous_outflow,
                "instantaneous_signed_error": instantaneous_outflow - ref_instantaneous_outflow,
                "cumulative_actual": profile.cumulative_outflow,
                "cumulative_reference": ref_boundary_outflow,
                "cumulative_signed_error": profile.cumulative_outflow - ref_boundary_outflow,
            },
        }));
    }
    Ok(rows)
}

fn actual_f32_mesh(n: usize) -> Vec<f32> {
    // Same conversion as `advance_fortran_compatible_step`: reversed public
    // coordinates are translated to kernel coordinates in f32.
    let coordinates = coordinates(n);
    let last_x = coordinates[n - 1];
    let mut kernel = vec![0.0_f32; n];
    for (i, coordinate) in coordinates.iter().enumerate() {
        let k = n - 1 - i;
        kernel[k] = (last_x - coordinate) as f32;
    }
    kernel
}

fn coefficient_and_matrix_diagnostic(n: usize) -> Result<Value, String> {
    let x = actual_f32_mesh(n);
    let velocity = (-Q) as f32;
    let water = THETA as f32;
    let dispersion = DISPERSION as f32;
    let retardation = (1.0 + RHO * KD / THETA) as f32;
    let mut elements = Vec::new();
    let mut wc_baseline = vec![0.0_f32; n];
    let mut wc_stable = vec![0.0_f32; n];
    for i in 0..n - 1 {
        let dx = x[i + 1] - x[i];
        let vv1 = (velocity / water + velocity / water) / 2.0;
        let dd = (dispersion + dispersion) / 2.0;
        let pe2 = if dd > 0.0 { dx * vv1 / dd / 2.0 } else { 11.0 };
        let pc = pecour(PecletInputs {
            dx,
            dt: 1.0 / 128.0,
            velocity,
            neighbor_velocity: velocity,
            water,
            neighbor_water: water,
            dispersion,
            neighbor_dispersion: dispersion,
            retardation,
            neighbor_retardation: retardation,
            upstream_weighting: true,
            artificial_dispersion: false,
            level_is_new: false,
            iteration: 1,
            epsi: 0.5,
            threshold: 0.0,
        });
        let baseline = pc.weight.ok_or("missing baseline weight")?;
        let pe = pe2 as f64;
        let stable = (1.0 / pe.tanh() - 1.0 / pe).clamp(-1.0, 1.0) as f32;
        wc_baseline[i] = baseline;
        wc_stable[i] = stable;
        elements.push(json!({
            "element": i,
            "x_left_f32": x[i],
            "x_right_f32": x[i + 1],
            "dx_f32": dx,
            "pe2_f32": pe2,
            "wc_baseline_f32": baseline,
            "wc_stable_f64_expression_f32": stable,
            "difference_stable_minus_baseline": stable - baseline,
        }));
    }
    let max_weight_difference = elements
        .iter()
        .map(|row| {
            row["difference_stable_minus_baseline"]
                .as_f64()
                .unwrap_or(0.0)
                .abs()
        })
        .fold(0.0_f64, f64::max);

    let assemble = |weights: &[f32]| -> Result<MatrixRows, String> {
        let mut lower = vec![0.0_f64; n];
        let mut diagonal = vec![0.0_f64; n];
        let mut upper = vec![0.0_f64; n];
        let mut rhs = vec![0.0_f64; n];
        assemble_interior_into(
            AssemblyInputs {
                x: &x,
                theta_old: &vec![water; n],
                theta_new: &vec![water; n],
                velocity_old: &vec![velocity; n],
                velocity_new: &vec![velocity; n],
                concentration: &vec![1.0_f32; n],
                dispersion: &vec![dispersion; n],
                retardation: &vec![retardation; n],
                upstream_weight: weights,
                g0: &vec![0.0_f32; n],
                g1: &vec![0.0_f32; n],
            },
            AssemblyLevel::New,
            0.5,
            0.5,
            1.0 / 128.0,
            &mut lower,
            &mut diagonal,
            &mut upper,
            &mut rhs,
        )
        .map_err(str::to_string)?;
        Ok((lower, diagonal, upper, rhs))
    };
    let baseline = assemble(&wc_baseline)?;
    let stable = assemble(&wc_stable)?;
    let interior = 1..n - 1;
    let mut max_operator_difference = 0.0_f64;
    let mut max_row_sum_difference = 0.0_f64;
    for i in interior {
        for (left, right) in [
            (baseline.0[i], stable.0[i]),
            (baseline.1[i], stable.1[i]),
            (baseline.2[i], stable.2[i]),
            (baseline.3[i], stable.3[i]),
        ] {
            max_operator_difference = max_operator_difference.max((left - right).abs());
        }
        let baseline_row_sum = baseline.0[i] + baseline.1[i] + baseline.2[i];
        let stable_row_sum = stable.0[i] + stable.1[i] + stable.2[i];
        max_row_sum_difference =
            max_row_sum_difference.max((baseline_row_sum - stable_row_sum).abs());
    }

    Ok(json!({
        "nodes": n,
        "element_count": n - 1,
        "actual_f32_mesh": x,
        "elements": elements,
        "max_abs_weight_difference": max_weight_difference,
        "transport_operator_new_level": {
            "scope": "interior rows only; first-step actual f32 coefficients; g0=g1=0; dense source terms excluded",
            "max_abs_coefficient_or_rhs_difference": max_operator_difference,
            "max_abs_row_sum_difference": max_row_sum_difference,
        },
    }))
}

#[test]
#[ignore = "bounded diagnostic audit; run explicitly"]
fn robin_coefficient_error_audit() -> Result<(), String> {
    let mut runs = Vec::new();
    for (n, dt) in CONFIGS {
        let reference = load_reference(n)?;
        for (variant, force_upstream, stable) in [
            ("public_baseline_no_upwind", false, false),
            ("forced_upstream_source_weight", true, false),
            ("forced_upstream_stable_weight", true, true),
        ] {
            let run = simulate(n, dt, force_upstream, stable)?;
            runs.push(json!({
                "nodes": n,
                "dt": dt,
                "variant": variant,
                "weight_calls": run.weight_calls,
                "profiles": error_summary(&run, &reference)?,
            }));
        }
    }
    let diagnostics = [129_usize, 193, 257]
        .iter()
        .map(|n| coefficient_and_matrix_diagnostic(*n))
        .collect::<Result<Vec<_>, _>>()?;
    let payload = json!({
        "schema": "robin-coefficient-error-audit-v1",
        "date": "2026-11-11",
        "scope": "diagnostic only; no production numerical change",
        "configurations": CONFIGS,
        "runs": runs,
        "coefficient_and_operator_diagnostics": diagnostics,
    });
    fs::create_dir_all(output_root()).map_err(|e| e.to_string())?;
    let path = output_root().join("diagnostic_runs.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
