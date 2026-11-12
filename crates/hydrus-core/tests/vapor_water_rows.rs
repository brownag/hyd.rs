use hydrus_core::heat::vapor::water::{
    assemble_water_rows, phase_velocities, PhaseVelocityInput, VaporWaterRowsInput,
};
use hydrus_core::heat::vapor::{VaporError, VaporProperties};

fn properties(head: &[f64], thermal: &[f64]) -> VaporProperties {
    VaporProperties {
        liquid_conductivities: vec![2.0; head.len()],
        liquid_thermal_conductivities: vec![0.0; head.len()],
        vapor_head_conductivities: head.to_vec(),
        vapor_thermal_conductivities: thermal.to_vec(),
        vapor_contents: vec![0.0; head.len()],
        effective_hydraulic_capacities: vec![0.2; head.len()],
    }
}

#[test]
fn three_node_nonuniform_row_matches_hand_calculation() {
    let vapor = properties(&[1.0, 2.0, 3.0], &[0.5, 1.0, 2.0]);
    let rows = assemble_water_rows(&VaporWaterRowsInput {
        coordinates: &[0.0, 2.0, 5.0],
        gravity: 1.0,
        liquid_conductivities: &[2.0, 4.0, 6.0],
        effective_capacities: &[0.1, 0.2, 0.3],
        heads_new: &[0.0, -1.0, -2.0],
        liquid_water_old: &[0.2; 3],
        liquid_water_new: &[0.2, 0.21, 0.2],
        vapor_water_old: &[0.01, 0.02, 0.03],
        vapor_water_new: &[0.02, 0.01, 0.05],
        temperatures: &[10.0, 20.0, 35.0],
        sink: &[0.0; 3],
        timestep: 2.0,
        vapor: &vapor,
    })
    .unwrap();

    // Vapor face conductivities are 1.5/2.5; total face conductivities
    // are 4.5/7.5. Width=2.5, storage coefficient=.25. Liquid gravity
    // remains based on liquid faces 3/5. Thermal RHS=7.5-3.75 and vapor
    // storage contributes +.0125. All values are direct source-algebra arithmetic.
    let row = rows[0];
    assert_eq!(row.lower, -2.25);
    assert_eq!(row.diagonal, 5.0);
    assert_eq!(row.upper, -2.5);
    assert!((row.rhs - 5.5).abs() <= 2.0 * f64::EPSILON * 5.5);
}

#[test]
fn five_node_zero_vapor_row_reduces_to_liquid_reset_row() {
    let vapor = properties(&[0.0; 5], &[0.0; 5]);
    let rows = assemble_water_rows(&VaporWaterRowsInput {
        coordinates: &[0.0, 1.0, 3.0, 6.0, 10.0],
        gravity: 0.0,
        liquid_conductivities: &[2.0; 5],
        effective_capacities: &[1.0; 5],
        heads_new: &[0.0; 5],
        liquid_water_old: &[0.2; 5],
        liquid_water_new: &[0.2; 5],
        vapor_water_old: &[0.0; 5],
        vapor_water_new: &[0.0; 5],
        temperatures: &[20.0; 5],
        sink: &[0.0; 5],
        timestep: 2.0,
        vapor: &vapor,
    })
    .unwrap();
    assert_eq!(rows.len(), 3);
    let row = rows[1]; // dxA=2, dxB=3, width=2.5
    assert_eq!(row.lower, -1.0);
    assert!((row.diagonal - (1.0 + 2.0 / 3.0 + 1.25)).abs() < 1.0e-15);
    assert_eq!(row.upper, -2.0 / 3.0);
    assert_eq!(row.rhs, 0.0);
}

#[test]
fn phase_velocity_components_keep_gravity_and_endpoint_storage_separate() {
    let vapor = properties(&[1.0, 2.0, 3.0], &[0.5, 1.0, 1.5]);
    let values = phase_velocities(&PhaseVelocityInput {
        coordinates: &[0.0, 1.0, 3.0],
        gravity: 0.5,
        heads: &[0.0, -1.0, -3.0],
        liquid_conductivities: &[2.0; 3],
        liquid_water_old: &[0.2; 3],
        liquid_water_new: &[0.2; 3],
        sink: &[0.0; 3],
        temperatures: &[10.0, 20.0, 40.0],
        vapor_water_old: &[0.0; 3],
        vapor_water_new: &[0.0; 3],
        timestep: 2.0,
        vapor: &vapor,
    })
    .unwrap();
    assert_eq!(values.liquid, vec![1.0, 1.0, 1.0]);
    assert_eq!(values.vapor[0], -6.0);
    assert_eq!(values.vapor[1], -22.0 / 3.0);
    assert_eq!(values.vapor[2], -10.0);
    assert_eq!(values.total[0], -5.0);

    let endpoint_storage = phase_velocities(&PhaseVelocityInput {
        vapor_water_old: &[0.0; 3],
        vapor_water_new: &[0.2, 0.0, 0.2],
        ..PhaseVelocityInput {
            coordinates: &[0.0, 1.0, 3.0],
            gravity: 0.5,
            heads: &[0.0, -1.0, -3.0],
            liquid_conductivities: &[2.0; 3],
            liquid_water_old: &[0.2; 3],
            liquid_water_new: &[0.2; 3],
            sink: &[0.0; 3],
            temperatures: &[10.0, 20.0, 40.0],
            vapor_water_old: &[0.0; 3],
            vapor_water_new: &[0.0; 3],
            timestep: 2.0,
            vapor: &vapor,
        }
    })
    .unwrap();
    assert_eq!(endpoint_storage.vapor[0], -5.95);
    assert_eq!(endpoint_storage.vapor[2], -10.1);
}

#[test]
fn malformed_water_inputs_are_rejected_before_rows_are_returned() {
    let vapor = properties(&[0.0; 3], &[0.0; 3]);
    let bad = assemble_water_rows(&VaporWaterRowsInput {
        coordinates: &[0.0, 0.0, 1.0],
        gravity: 1.0,
        liquid_conductivities: &[1.0; 3],
        effective_capacities: &[1.0; 3],
        heads_new: &[0.0; 3],
        liquid_water_old: &[0.2; 3],
        liquid_water_new: &[0.2; 3],
        vapor_water_old: &[0.0; 3],
        vapor_water_new: &[0.0; 3],
        temperatures: &[20.0; 3],
        sink: &[0.0; 3],
        timestep: 1.0,
        vapor: &vapor,
    });
    assert!(matches!(
        bad,
        Err(VaporError::InvalidValue {
            name: "coordinates",
            ..
        })
    ));

    let bad = phase_velocities(&PhaseVelocityInput {
        coordinates: &[0.0, 1.0, 2.0],
        gravity: 1.0,
        heads: &[0.0; 3],
        liquid_conductivities: &[1.0; 3],
        liquid_water_old: &[0.2; 3],
        liquid_water_new: &[0.2; 3],
        sink: &[0.0; 3],
        temperatures: &[20.0; 2],
        vapor_water_old: &[0.0; 3],
        vapor_water_new: &[0.0; 3],
        timestep: 1.0,
        vapor: &vapor,
    });
    assert!(matches!(bad, Err(VaporError::LengthMismatch { .. })));
}
