#![allow(dead_code)]

use hydrus_core::hydraulics::Mesh1D;
use hydrus_core::WaterStepPolicy;

pub fn mesh() -> Mesh1D {
    Mesh1D::new(vec![0.0, 1.0, 2.0], vec![0; 3], None, 1, 0, 0.0).unwrap()
}

pub fn policy(max_iterations: usize, minimum_timestep: f64) -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations,
        head_tolerance: 0.5,
        water_tolerance: 0.5,
        minimum_timestep,
        retry_factor: 3.0,
        minimum_pivot: 1e-12,
    }
}

pub fn default_policy() -> WaterStepPolicy {
    WaterStepPolicy {
        max_iterations: 8,
        head_tolerance: 1e-10,
        water_tolerance: 1e-10,
        minimum_timestep: 0.01,
        retry_factor: 2.0,
        minimum_pivot: 1e-12,
    }
}
