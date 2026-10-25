//! Pinning test for SD-005 (`docs/FORTRAN_SOURCE_DEFECTS.md`).
//!
//! SD-005: `SOLUTE.FOR:816-818` (node-1 row for `kBotCh=-1` with `vN(1)<=0`)
//! assembles `c1=c2` instead of the natural flux row used by the same BC code
//! when `vN(1)>0` (`SOLUTE.FOR:811-814`). For the `diffusion_cosine` case this
//! row is reached on every step, so the dissolved-plus-sorbed inventory of a
//! zero-flux diffusion problem drifts downward. The 22.4.2 study measured the
//! drift at about -3.8e-3 absolute (about 2.2e-3 relative) at 17 nodes and
//! t=1.0 (`planning/master/M5V_STUDY_22_DIFFUSION.md:115`).
//!
//! This test pins the defect. It fails if the end row is made conservative,
//! because the relative inventory loss then falls below the threshold.
use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::solute::{advance_solute_step_with_processes, SoluteProcessInput};
use hydrus_core::{
    AcceptedWaterSnapshot, EquilibriumSorption, SoluteBackend, SoluteBoundaries,
    SoluteBoundaryMode as Boundary, SoluteConfig, SoluteSpecies, SoluteState, SoluteTransportInput,
    SoluteUnits, SoluteWorkspace,
};

const NODES: usize = 17;
const LENGTH: f64 = 2.0;
const THETA: f64 = 0.43;
const RHO: f64 = 0.8;
const KD: f64 = 0.5375;
const DISPERSION: f64 = 0.2;
const DT: f64 = 0.0078125;
const STEPS: usize = 128;

/// Trapezoid-weighted dissolved plus linear sorbed inventory.
fn inventory(concentration: &[f64], coordinates: &[f64]) -> f64 {
    let storage = THETA + RHO * KD;
    coordinates
        .windows(2)
        .zip(concentration.windows(2))
        .map(|(x, c)| 0.5 * (x[1] - x[0]) * (c[0] + c[1]))
        .sum::<f64>()
        * storage
}

fn run_diffusion_cosine() -> Result<(f64, f64), String> {
    let dx = LENGTH / (NODES - 1) as f64;
    let coordinates: Vec<f64> = (0..NODES).map(|i| i as f64 * dx).collect();
    let mesh = Mesh1D::new(coordinates.clone(), vec![0; NODES], None, 1, 0, 0.0)
        .map_err(|e| e.to_string())?;
    let species = vec![SoluteSpecies {
        name: "S1".into(),
        decay_rate: 0.0,
        equilibrium_sorption: EquilibriumSorption::Linear {
            distribution_coefficient: KD,
        },
        kinetic_sorption: None,
    }];
    // Inlet total flux of zero (`kBotCh=-1`, `vN(1)=0`); outlet advective outflow (`kTopCh=0`).
    let boundaries = vec![SoluteBoundaries {
        lower: Boundary::TotalFlux { flux: 0.0 },
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

    // Cosine initial profile with mean 1, amplitude 0.25, mode 1.
    let initial: Vec<f64> = coordinates
        .iter()
        .map(|x| 1.0 + 0.25 * (std::f64::consts::PI * x / LENGTH).cos())
        .collect();
    let mut state = SoluteState::new(&config, &mesh, 0.0, initial.clone(), vec![0.0; NODES])
        .map_err(|e| e.to_string())?;
    let mut workspace = SoluteWorkspace::new(NODES, 1).map_err(|e| e.to_string())?;
    let theta = vec![THETA; NODES];
    let flux = vec![0.0; NODES + 1];
    let dispersion = vec![DISPERSION; NODES - 1];
    let density = vec![RHO; NODES];

    let initial_inventory = inventory(&initial, &coordinates);
    for step in 1..=STEPS {
        advance_solute_step_with_processes(
            &mut state,
            &mut workspace,
            &mesh,
            &config,
            SoluteTransportInput {
                accepted_water: AcceptedWaterSnapshot {
                    accepted_time: step as f64 * DT,
                    water_contents: &theta,
                    face_fluxes: &flux,
                },
                start_water_contents: &theta,
                face_dispersion: &dispersion,
                timestep: DT,
                minimum_pivot: 1e-12,
            },
            Some(SoluteProcessInput {
                bulk_density: &density,
                max_iterations: 20,
                nonlinear_tolerance: 1e-6,
            }),
        )
        .map_err(|e| format!("step {step}: {e}"))?;
    }
    let final_inventory = inventory(state.concentrations(), &coordinates);
    Ok((initial_inventory, final_inventory))
}

/// SD-005 (`docs/FORTRAN_SOURCE_DEFECTS.md`): the zero-flux node-1 `c1=c2` row at
/// `SOLUTE.FOR:816-818` loses inventory in `diffusion_cosine`. Fails if the row is made
/// conservative, because the relative loss then drops below 1e-4.
#[test]
fn source_defect_sd005_diffusion_end_row_loses_inventory() {
    let (initial, final_) = run_diffusion_cosine().expect("diffusion run");
    let relative_loss = (initial - final_) / initial;
    assert!(
        relative_loss > 1e-4,
        "SD-005 not reproduced: relative inventory loss {relative_loss:e} (initial {initial}, final {final_})"
    );
}
