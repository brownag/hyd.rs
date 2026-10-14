//! The water step publishes the accepted water snapshot for the solute step.
//!
//! Units follow the water tests: cm, days, cm/day. Face fluxes are positive
//! toward increasing coordinate (toward the upper boundary); balance terms are
//! positive when water enters the soil. The model is an independently built
//! exponential-conductivity test fixture, not a Fortran or manual result.
use hydrus_core::hydraulics::{
    BoundaryRow, Evaluation, FaceAveraging, HydraulicDerivatives, HydraulicModel, Mesh1D,
};
use hydrus_core::{
    advance_atmospheric_step, advance_solute_step, advance_water_step, AcceptedWaterSnapshot,
    AtmosphericRates, EquilibriumSorption, ForcingSchedule, ForcingUnits, HydraulicError,
    LowerBoundaryMode, SoluteBoundaries, SoluteBoundaryMode, SoluteConfig, SoluteSpecies,
    SoluteState, SoluteTransportInput, SoluteUnits, SoluteWorkspace, UpperBoundary, WaterState,
    WaterStepInput, WaterStepPolicy, WaterWorkspace,
};

/// Linear water retention with unit conductivity, so the caller-supplied upper
/// half-cell row below is exactly the discrete balance of the last cell.
struct LinearModel;

impl HydraulicModel for LinearModel {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: 1.0 + 0.1 * head,
            conductivity: 1.0,
            capacity: 0.1,
        })
    }

    fn derivatives(&self, _head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Ok(HydraulicDerivatives {
            dtheta_dhead: 0.1,
            dconductivity_dhead: 0.0,
            dcapacity_dhead: 0.0,
        })
    }
}

struct ExponentialModel;

impl HydraulicModel for ExponentialModel {
    fn evaluate(&self, head: f64) -> Result<Evaluation, HydraulicError> {
        Ok(Evaluation {
            theta: 1.0 + 0.1 * head,
            conductivity: (0.05 * head).exp(),
            capacity: 0.1,
        })
    }

    fn derivatives(&self, head: f64) -> Result<HydraulicDerivatives, HydraulicError> {
        Ok(HydraulicDerivatives {
            dtheta_dhead: 0.1,
            dconductivity_dhead: 0.05 * (0.05 * head).exp(),
            dcapacity_dhead: 0.0,
        })
    }
}

fn policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 30,
        head_tolerance: 1e-12,
        water_tolerance: 1e-12,
        minimum_timestep: 0.001,
        retry_factor: 2.0,
        minimum_pivot: 1e-12,
    }
}

const COORDINATES: [f64; 5] = [0.0, 0.5, 1.5, 2.0, 3.0];
const HEADS: [f64; 5] = [-4.0, -2.5, -3.0, -1.0, -2.0];

fn sloped_mesh() -> Mesh1D {
    Mesh1D::new(COORDINATES.to_vec(), vec![0; 5], None, 1, 0, 0.8).unwrap()
}

fn initial_state() -> WaterState {
    let water = HEADS.iter().map(|head| 1.0 + 0.1 * head).collect();
    WaterState::new(0.0, HEADS.to_vec(), water).unwrap()
}

fn control_width(node: usize) -> f64 {
    let below = if node == 0 {
        0.0
    } else {
        COORDINATES[node] - COORDINATES[node - 1]
    };
    let above = if node == 4 {
        0.0
    } else {
        COORDINATES[node + 1] - COORDINATES[node]
    };
    (below + above) / 2.0
}

/// Upper half-cell balance for `LinearModel` (K = 1, slope cosine 0.8, dt = 0.2);
/// the transform adds the prescribed inflow to `rhs`.
fn upper_row(sink: &[f64]) -> BoundaryRow {
    let dx = COORDINATES[4] - COORDINATES[3];
    let storage = control_width(4) * 0.1 / 0.2;
    BoundaryRow {
        lower: -1.0 / dx,
        diagonal: storage + 1.0 / dx,
        upper: 0.0,
        rhs: storage * HEADS[4] - 0.8 - sink[4] * control_width(4),
    }
}

fn input<'a>(
    mesh: &'a Mesh1D,
    materials: &'a [&'a dyn HydraulicModel],
    sink: &'a [f64],
    averaging: FaceAveraging,
) -> WaterStepInput<'a> {
    WaterStepInput {
        mesh,
        materials,
        sink,
        root_uptake: None,
        timestep: 0.2,
        target_time: 0.2,
        face_averaging: averaging,
        lower: LowerBoundaryMode::PrescribedFlux { flux: 0.3 },
        upper: UpperBoundary::Flux {
            flux: -0.1,
            row: upper_row(sink),
        },
    }
}

fn schedule(precipitation: f64, evaporation: f64) -> ForcingSchedule<'static> {
    ForcingSchedule::new(
        AtmosphericRates {
            precipitation,
            evaporation,
        },
        vec![],
        ForcingUnits {
            time: "day",
            water_depth_rate: "cm/day",
        },
    )
    .unwrap()
}

#[test]
fn endpoint_fluxes_equal_the_balance_boundary_fluxes_bitwise() {
    let mesh = sloped_mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 5];
    let mut state = initial_state();
    let mut workspace = WaterWorkspace::new(5).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(&mesh, &materials, &sink, FaceAveraging::Arithmetic),
        policy(),
    )
    .unwrap();
    let snapshot = report.accepted_water();
    assert_eq!(snapshot.face_fluxes.len(), 6);
    // Lower entry points toward +x; upper entry points against it.
    assert_eq!(snapshot.face_fluxes[0].to_bits(), 0.3f64.to_bits());
    assert_eq!(snapshot.face_fluxes[5].to_bits(), 0.1f64.to_bits());
    let dt = report.timestep;
    let net = (snapshot.face_fluxes[0] - snapshot.face_fluxes[5]) * dt;
    assert_eq!(net.to_bits(), report.balance.boundary_flux.to_bits());
    assert_eq!(net.to_bits(), report.balance.soil_boundary_flux.to_bits());
}

#[test]
fn atmospheric_upper_endpoint_is_the_soil_flux_not_the_surface_exchange() {
    let mesh = sloped_mesh();
    let model = ExponentialModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 5];
    let mut state = initial_state();
    let mut workspace = WaterWorkspace::with_transition_capacity(5, 30).unwrap();
    let mut step = input(&mesh, &materials, &sink, FaceAveraging::Arithmetic);
    step.upper = UpperBoundary::Atmospheric { critical_head: 0.1 };
    // Heavy rain ponds, so the soil flux differs from the surface exchange.
    let report = advance_atmospheric_step(
        &mut state,
        &mut workspace,
        step,
        policy(),
        &schedule(5.0, 0.0),
    )
    .unwrap();
    let faces = report.face_fluxes;
    let net = (faces[0] - faces[5]) * report.timestep;
    assert_eq!(net.to_bits(), report.balance.soil_boundary_flux.to_bits());
    assert_ne!(
        report.balance.soil_boundary_flux.to_bits(),
        report.balance.boundary_flux.to_bits()
    );
}

#[test]
fn boundary_flux_matches_storage_change_within_balance_tolerance() {
    let mesh = sloped_mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.01, 0.02, 0.0, 0.03, 0.01];
    let mut state = initial_state();
    let mut workspace = WaterWorkspace::new(5).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(&mesh, &materials, &sink, FaceAveraging::Geometric),
        policy(),
    )
    .unwrap();
    let faces = report.face_fluxes;
    let sink_total: f64 = (0..5).map(|node| sink[node] * control_width(node)).sum();
    let residual = (faces[0] - faces[5]) * report.timestep
        - sink_total * report.timestep
        - report.balance.soil_storage_change;
    assert!(residual.abs() < 1e-9, "residual {residual}");
}

fn assert_per_cell_balance(
    old_water: &[f64],
    new_water: &[f64],
    sink: &[f64],
    faces: &[f64],
    dt: f64,
) {
    for node in 0..5 {
        let width = control_width(node);
        let storage = (new_water[node] - old_water[node]) * width / dt + sink[node] * width;
        let net_inflow = faces[node] - faces[node + 1];
        assert!(
            (storage - net_inflow).abs() < 1e-9,
            "node {node}: storage+sink {storage} vs inflow {net_inflow}"
        );
    }
}

#[test]
fn per_cell_water_balance_closes_on_a_sloped_nonuniform_step() {
    let mesh = sloped_mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.01, 0.02, 0.0, 0.03, 0.01];
    for averaging in [FaceAveraging::Arithmetic, FaceAveraging::Geometric] {
        let mut state = initial_state();
        let old = state.water_contents().to_vec();
        let mut workspace = WaterWorkspace::new(5).unwrap();
        let report = advance_water_step(
            &mut state,
            &mut workspace,
            input(&mesh, &materials, &sink, averaging),
            policy(),
        )
        .unwrap();
        assert_per_cell_balance(
            &old,
            state.water_contents(),
            &sink,
            report.face_fluxes,
            report.timestep,
        );
    }
}

#[test]
fn per_cell_water_balance_closes_on_an_atmospheric_step() {
    let mesh = sloped_mesh();
    let model = ExponentialModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0, 0.01, 0.0, 0.02, 0.0];
    for (precipitation, evaporation) in [(0.0, 0.4), (5.0, 0.0)] {
        let mut state = initial_state();
        let old = state.water_contents().to_vec();
        let mut workspace = WaterWorkspace::with_transition_capacity(5, 30).unwrap();
        let mut step = input(&mesh, &materials, &sink, FaceAveraging::Arithmetic);
        step.upper = UpperBoundary::Atmospheric { critical_head: 0.1 };
        let report = advance_atmospheric_step(
            &mut state,
            &mut workspace,
            step,
            policy(),
            &schedule(precipitation, evaporation),
        )
        .unwrap();
        assert_per_cell_balance(
            &old,
            state.water_contents(),
            &sink,
            report.face_fluxes,
            report.timestep,
        );
    }
}

#[test]
fn published_snapshot_drives_a_solute_step() {
    let mesh = sloped_mesh();
    let model = LinearModel;
    let materials: [&dyn HydraulicModel; 1] = [&model];
    let sink = [0.0; 5];
    let mut state = initial_state();
    let old = state.water_contents().to_vec();
    let mut workspace = WaterWorkspace::new(5).unwrap();
    let report = advance_water_step(
        &mut state,
        &mut workspace,
        input(&mesh, &materials, &sink, FaceAveraging::Arithmetic),
        policy(),
    )
    .unwrap();
    let snapshot = report.accepted_water();
    let config = SoluteConfig::new(
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
        SoluteBoundaries {
            lower: SoluteBoundaryMode::TotalFlux { flux: 0.0 },
            upper: SoluteBoundaryMode::TotalFlux { flux: 0.0 },
        },
        1,
        &[],
    )
    .unwrap();
    let mut solute = SoluteState::new(&config, &mesh, 0.0, vec![1.0; 5], vec![0.0; 5]).unwrap();
    let mut solute_workspace = SoluteWorkspace::new(5, 1).unwrap();
    advance_solute_step(
        &mut solute,
        &mut solute_workspace,
        &mesh,
        &config,
        SoluteTransportInput {
            accepted_water: snapshot,
            start_water_contents: &old,
            face_dispersion: &[0.1; 4],
            timestep: report.timestep,
            minimum_pivot: 1e-12,
        },
    )
    .unwrap();
    assert_eq!(solute.time(), report.accepted_time);
}

#[test]
fn both_public_paths_name_the_same_snapshot_type() {
    let from_root: AcceptedWaterSnapshot<'static> = AcceptedWaterSnapshot {
        accepted_time: 0.0,
        water_contents: &[],
        face_fluxes: &[],
    };
    let from_solute: hydrus_core::solute::AcceptedWaterSnapshot<'static> = from_root;
    assert_eq!(from_solute.accepted_time, 0.0);
}
