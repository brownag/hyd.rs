use hydrus_core::heat::state::{HeatState, HeatStateError};

fn state() -> HeatState {
    HeatState::new(0.0, vec![10.0, 12.0, 14.0]).unwrap()
}

#[test]
fn construction_rejects_short_length() {
    assert!(matches!(
        HeatState::new(0.0, vec![1.0, 2.0]),
        Err(HeatStateError::InvalidState { .. })
    ));
}

#[test]
fn construction_rejects_non_finite_time_and_temperature() {
    assert_eq!(
        HeatState::new(f64::NAN, vec![1.0, 2.0, 3.0]),
        Err(HeatStateError::NonFiniteInput { name: "time" })
    );
    assert_eq!(
        HeatState::new(0.0, vec![1.0, f64::INFINITY, 3.0]),
        Err(HeatStateError::NonFiniteInput {
            name: "temperatures"
        })
    );
}

#[test]
fn construction_starts_trial_equal_to_accepted() {
    let state = state();
    assert_eq!(state.trial_temperatures(), state.temperatures());
    assert_eq!(state.time(), 0.0);
}

#[test]
fn accept_commits_trial_and_time() {
    let mut state = state();
    state
        .trial_temperatures_mut()
        .copy_from_slice(&[11.0, 13.0, 15.0]);
    state.accept(2.5).unwrap();
    assert_eq!(state.time(), 2.5);
    assert_eq!(state.temperatures(), &[11.0, 13.0, 15.0]);
    let snapshot = state.snapshot();
    assert_eq!(snapshot.accepted_time, 2.5);
    assert_eq!(snapshot.temperatures, &[11.0, 13.0, 15.0]);
}

#[test]
fn reject_leaves_accepted_state_unchanged() {
    let mut state = state();
    let original = state.clone();
    state
        .trial_temperatures_mut()
        .copy_from_slice(&[99.0, 99.0, 99.0]);
    state.reject();
    assert_eq!(state, original);
    assert_eq!(state.trial_temperatures(), original.temperatures());
}

#[test]
fn non_finite_trial_is_rejected_by_accept_without_mutation() {
    let mut state = state();
    let original = state.clone();
    state.trial_temperatures_mut()[1] = f64::NAN;
    assert_eq!(
        state.accept(1.0),
        Err(HeatStateError::NonFiniteInput {
            name: "trial_temperatures"
        })
    );
    assert_eq!(state.time(), original.time());
    assert_eq!(state.temperatures(), original.temperatures());
}

#[test]
fn non_finite_accepted_time_is_rejected_without_mutation() {
    let mut state = state();
    let original = state.clone();
    state
        .trial_temperatures_mut()
        .copy_from_slice(&[1.0, 2.0, 3.0]);
    assert_eq!(
        state.accept(f64::INFINITY),
        Err(HeatStateError::NonFiniteInput {
            name: "accepted_time"
        })
    );
    assert_eq!(state.time(), original.time());
    assert_eq!(state.temperatures(), original.temperatures());
}
