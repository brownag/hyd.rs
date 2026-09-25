use hydrus_core::hydraulics::{Mesh1D, TridiagonalError};
use hydrus_core::{
    advance_solute_step, AcceptedWaterSnapshot, EquilibriumSorption, SoluteBoundaries,
    SoluteBoundaryMode as Boundary, SoluteConfig, SoluteSpecies, SoluteState, SoluteStepError,
    SoluteTransportInput, SoluteUnits, SoluteWorkspace,
};

fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
}

fn config(lower: Boundary, upper: Boundary) -> SoluteConfig {
    SoluteConfig::new(
        SoluteUnits {
            concentration: "mass/volume".into(),
            sorbed_mass: "mass/mass".into(),
        },
        vec![SoluteSpecies {
            name: "solute".into(),
            decay_rate: 0.0,
            equilibrium_sorption: EquilibriumSorption::None,
            kinetic_sorption: None,
        }],
        SoluteBoundaries { lower, upper },
        1,
        &[],
    )
    .unwrap()
}

fn state(c: [f64; 3]) -> SoluteState {
    SoluteState::new(
        &config(
            Boundary::TotalFlux { flux: 0.0 },
            Boundary::TotalFlux { flux: 0.0 },
        ),
        &mesh(),
        0.0,
        c.to_vec(),
        vec![0.0; 3],
    )
    .unwrap()
}

fn run(
    state: &mut SoluteState,
    config: &SoluteConfig,
    water: [f64; 3],
    fluxes: [f64; 4],
    dispersion: [f64; 2],
    timestep: f64,
    minimum_pivot: f64,
) -> Result<(), SoluteStepError> {
    let mesh = mesh();
    let mut workspace = SoluteWorkspace::new(3, 1).unwrap();
    let accepted_time = state.time() + timestep;
    advance_solute_step(
        state,
        &mut workspace,
        &mesh,
        config,
        SoluteTransportInput {
            accepted_water: AcceptedWaterSnapshot {
                accepted_time,
                water_contents: &water,
                face_fluxes: &fluxes,
            },
            face_dispersion: &dispersion,
            timestep,
            minimum_pivot,
        },
    )
}

fn mass(state: &SoluteState, water: [f64; 3]) -> f64 {
    let widths = [0.5, 1.0, 0.5];
    state
        .concentrations()
        .iter()
        .zip(water)
        .zip(widths)
        .map(|((&concentration, theta), width)| concentration * theta * width)
        .sum()
}

fn close(left: f64, right: f64) {
    assert!((left - right).abs() <= 1e-12, "{left} != {right}");
}

#[test]
fn zero_flow_diffusion_changes_profile_and_conserves_closed_column_mass() {
    let water = [0.3, 0.3, 0.3];
    let config = config(
        Boundary::TotalFlux { flux: 0.0 },
        Boundary::TotalFlux { flux: 0.0 },
    );
    let mut state = state([1.0, 0.0, 0.0]);
    let initial_mass = mass(&state, water);
    run(&mut state, &config, water, [0.0; 4], [0.2; 2], 0.1, 1e-12).unwrap();
    assert!(state.concentrations()[1] > 0.0);
    assert!(state.concentrations()[0] < 1.0);
    close(mass(&state, water), initial_mass);
}

#[test]
fn signed_advection_moves_mass_in_both_directions() {
    let water = [0.3; 3];
    let closed = config(
        Boundary::TotalFlux { flux: 0.0 },
        Boundary::TotalFlux { flux: 0.0 },
    );
    let mut rightward = state([1.0, 0.0, 0.0]);
    let right_mass = mass(&rightward, water);
    run(
        &mut rightward,
        &closed,
        water,
        [0.0, 0.2, 0.2, 0.0],
        [0.0; 2],
        0.1,
        1e-12,
    )
    .unwrap();
    assert!(rightward.concentrations()[1] > 0.0);
    assert!(rightward.concentrations()[0] < 1.0);
    close(mass(&rightward, water), right_mass);

    let mut leftward = state([0.0, 0.0, 1.0]);
    let left_mass = mass(&leftward, water);
    run(
        &mut leftward,
        &closed,
        water,
        [0.0, -0.2, -0.2, 0.0],
        [0.0; 2],
        0.1,
        1e-12,
    )
    .unwrap();
    assert!(leftward.concentrations()[1] > 0.0);
    assert!(leftward.concentrations()[2] < 1.0);
    close(mass(&leftward, water), left_mass);
}

#[test]
fn concentration_boundary_rows_apply_at_lower_upper_and_mixed_endpoints() {
    for (lower, upper, expected_node, expected_value) in [
        (
            Boundary::Concentration { value: 2.0 },
            Boundary::TotalFlux { flux: 0.0 },
            0,
            2.0,
        ),
        (
            Boundary::TotalFlux { flux: 0.0 },
            Boundary::Concentration { value: 3.0 },
            2,
            3.0,
        ),
        (
            Boundary::Concentration { value: 4.0 },
            Boundary::TotalFlux { flux: 0.0 },
            0,
            4.0,
        ),
        (
            Boundary::TotalFlux { flux: 0.0 },
            Boundary::Concentration { value: 5.0 },
            2,
            5.0,
        ),
    ] {
        let config = config(lower, upper);
        let mut state = state([0.2; 3]);
        run(
            &mut state, &config, [0.3; 3], [0.0; 4], [0.1; 2], 0.1, 1e-12,
        )
        .unwrap();
        close(state.concentrations()[expected_node], expected_value);
    }
}

#[test]
fn prescribed_total_flux_at_each_endpoint_changes_mass_with_inward_sign() {
    let water = [0.3; 3];
    for (lower_flux, upper_flux) in [(0.4, 0.0), (0.0, 0.6)] {
        let config = config(
            Boundary::TotalFlux { flux: lower_flux },
            Boundary::TotalFlux { flux: upper_flux },
        );
        let mut state = state([0.0; 3]);
        let before = mass(&state, water);
        let timestep = 0.1;
        run(
            &mut state, &config, water, [0.0; 4], [0.0; 2], timestep, 1e-12,
        )
        .unwrap();
        close(
            mass(&state, water) - before,
            timestep * (lower_flux + upper_flux),
        );
    }
}

#[test]
fn invalid_inputs_and_pivot_failure_leave_accepted_state_unchanged() {
    let mesh = mesh();
    let config = config(
        Boundary::TotalFlux { flux: 0.0 },
        Boundary::TotalFlux { flux: 0.0 },
    );
    let water = [0.3; 3];
    let mut state = state([0.2, 0.4, 0.6]);
    let original = state.clone();
    let mut workspace = SoluteWorkspace::new(3, 1).unwrap();

    for (water_contents, face_fluxes, dispersion) in [
        (&water[..2], &[0.0, 0.0, 0.0, 0.0][..], &[0.0, 0.0][..]),
        (&water[..], &[0.0, f64::NAN, 0.0, 0.0][..], &[0.0, 0.0][..]),
        (
            &water[..],
            &[0.0, 0.0, 0.0, 0.0][..],
            &[0.0, f64::INFINITY][..],
        ),
    ] {
        assert!(advance_solute_step(
            &mut state,
            &mut workspace,
            &mesh,
            &config,
            SoluteTransportInput {
                accepted_water: AcceptedWaterSnapshot {
                    accepted_time: 0.1,
                    water_contents,
                    face_fluxes
                },
                face_dispersion: dispersion,
                timestep: 0.1,
                minimum_pivot: 1e-12,
            },
        )
        .is_err());
        assert_eq!(state, original);
    }

    let pivot_error = advance_solute_step(
        &mut state,
        &mut workspace,
        &mesh,
        &config,
        SoluteTransportInput {
            accepted_water: AcceptedWaterSnapshot {
                accepted_time: 0.1,
                water_contents: &water,
                face_fluxes: &[0.0; 4],
            },
            face_dispersion: &[0.0; 2],
            timestep: 0.1,
            minimum_pivot: 10.0,
        },
    )
    .unwrap_err();
    assert!(matches!(
        pivot_error,
        SoluteStepError::Linear(TridiagonalError::SingularPivot { .. })
    ));
    assert_eq!(state, original);
}
