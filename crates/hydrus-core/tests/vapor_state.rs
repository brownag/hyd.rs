use hydrus_core::heat::vapor::state::{VaporState, VaporStateInput};
use hydrus_core::heat::vapor::VaporPrimitiveInput;

fn state() -> VaporState {
    let heads = [-100.0, -200.0, -300.0];
    let temperatures = [10.0, 20.0, 30.0];
    let theta = [0.2, 0.25, 0.3];
    let conductivity = [1.0e-6, 2.0e-6, 3.0e-6];
    let capacity = [0.001; 3];
    let materials = [0; 3];
    let porosity = [0.45];
    let predictor = [-90.0, -190.0, -290.0];
    VaporState::new(VaporStateInput {
        primitive: VaporPrimitiveInput {
            heads: &heads,
            temperatures: &temperatures,
            liquid_water_contents: &theta,
            liquid_conductivities: &conductivity,
            liquid_capacities: &capacity,
            material_indices: &materials,
            saturated_water_contents: &porosity,
            x_conv: 1.0,
            t_conv: 1.0,
        },
        coordinates: &[0.0, 0.5, 1.0],
        gravity: 1.0,
        initial_head_predictor: &predictor,
        initialization_timestep: 0.25,
        time: 2.0,
    })
    .unwrap()
}

#[test]
fn initialization_calculates_nonzero_vapor_and_keeps_predictor_separate() {
    let state = state();
    let accepted = state.snapshot();
    assert_eq!(accepted.time, 2.0);
    assert_eq!(accepted.previous_timestep, Some(0.25));
    assert_ne!(accepted.heads, accepted.head_predictor);
    assert!(accepted.vapor_contents.iter().any(|value| *value > 0.0));
    assert!(accepted.liquid_velocities.iter().any(|value| *value != 0.0));
}

#[test]
fn stage_accept_and_reject_preserve_accepted_history_atomically() {
    let mut state = state();
    let (
        original_time,
        original_heads,
        original_theta,
        original_temp,
        original_vapor,
        original_liquid_v,
        original_vapor_v,
        original_predictor,
    ) = {
        let original = state.snapshot();
        (
            original.time,
            original.heads.to_vec(),
            original.liquid_water_contents.to_vec(),
            original.temperatures.to_vec(),
            original.vapor_contents.to_vec(),
            original.liquid_velocities.to_vec(),
            original.vapor_velocities.to_vec(),
            original.head_predictor.to_vec(),
        )
    };
    let new_heads = [-95.0, -180.0, -275.0];
    let new_theta = [0.21, 0.26, 0.31];
    let new_temp = [11.0, 21.0, 31.0];
    let new_vapor = [0.001, 0.002, 0.003];
    let new_liquid_v = [1.0, 2.0, 3.0];
    let new_vapor_v = [-1.0, -2.0, -3.0];
    let predictor = [-94.0, -179.0, -274.0];

    state
        .stage(
            2.5,
            0.5,
            &new_heads,
            &new_theta,
            &new_temp,
            &new_vapor,
            &new_liquid_v,
            &new_vapor_v,
            &predictor,
        )
        .unwrap();
    assert!(state.has_staged_candidate());
    assert_eq!(state.snapshot().heads, original_heads);
    assert_eq!(state.staged_snapshot().unwrap().heads, new_heads);
    assert_eq!(state.staged_snapshot().unwrap().head_predictor, predictor);

    // Invalid staging must not replace the previously validated trial.
    assert!(state
        .stage(
            3.0,
            0.5,
            &new_heads[..2],
            &new_theta,
            &new_temp,
            &new_vapor,
            &new_liquid_v,
            &new_vapor_v,
            &predictor,
        )
        .is_err());
    assert_eq!(state.staged_snapshot().unwrap().heads, new_heads);

    state.reject();
    assert!(!state.has_staged_candidate());
    let after_reject = state.snapshot();
    assert_eq!(after_reject.time, original_time);
    assert_eq!(after_reject.heads, original_heads);
    assert_eq!(after_reject.liquid_water_contents, original_theta);
    assert_eq!(after_reject.temperatures, original_temp);
    assert_eq!(after_reject.vapor_contents, original_vapor);
    assert_eq!(after_reject.liquid_velocities, original_liquid_v);
    assert_eq!(after_reject.vapor_velocities, original_vapor_v);
    assert_eq!(after_reject.head_predictor, original_predictor);

    state
        .stage(
            2.5,
            0.5,
            &new_heads,
            &new_theta,
            &new_temp,
            &new_vapor,
            &new_liquid_v,
            &new_vapor_v,
            &predictor,
        )
        .unwrap();
    state.accept().unwrap();
    let accepted = state.snapshot();
    assert_eq!(accepted.time, 2.5);
    assert_eq!(accepted.previous_timestep, Some(0.5));
    assert_eq!(accepted.heads, new_heads);
    assert_eq!(accepted.liquid_water_contents, new_theta);
    assert_eq!(accepted.temperatures, new_temp);
    assert_eq!(accepted.vapor_contents, new_vapor);
    assert_eq!(accepted.liquid_velocities, new_liquid_v);
    assert_eq!(accepted.vapor_velocities, new_vapor_v);
    assert_eq!(accepted.head_predictor, predictor);
}
