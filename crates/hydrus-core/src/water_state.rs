use crate::WaterStepError;

/// Accepted single-domain water fields and accepted-only accounting.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterState {
    pub(crate) time: f64,
    pub(crate) heads: Vec<f64>,
    pub(crate) water_contents: Vec<f64>,
    pub(crate) accounting: WaterAccounting,
}

/// Counters and cumulative diagnostics changed only by accepted steps.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WaterAccounting {
    pub accepted_steps: u64,
    pub cumulative_iterations: u64,
    pub cumulative_storage_change: f64,
    pub cumulative_boundary_flux: f64,
    pub cumulative_sink: f64,
}

impl WaterState {
    pub fn new(
        time: f64,
        heads: Vec<f64>,
        water_contents: Vec<f64>,
    ) -> Result<Self, WaterStepError> {
        validate_finite("time", &[time])?;
        if heads.len() < 3 || heads.len() != water_contents.len() {
            return Err(WaterStepError::InvalidState {
                reason: "heads and water contents must have the same length of at least three",
            });
        }
        validate_finite("heads", &heads)?;
        validate_finite("water_contents", &water_contents)?;
        Ok(Self {
            time,
            heads,
            water_contents,
            accounting: WaterAccounting::default(),
        })
    }

    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn heads(&self) -> &[f64] {
        &self.heads
    }
    pub fn water_contents(&self) -> &[f64] {
        &self.water_contents
    }
    pub fn accounting(&self) -> WaterAccounting {
        self.accounting
    }
}

/// Fixed-size buffers owned by one water simulation.
#[derive(Clone, Debug)]
pub struct WaterWorkspace {
    pub(crate) lower: Vec<f64>,
    pub(crate) diagonal: Vec<f64>,
    pub(crate) upper: Vec<f64>,
    pub(crate) rhs: Vec<f64>,
    pub(crate) trial_heads: Vec<f64>,
    pub(crate) trial_water: Vec<f64>,
    snapshot_heads: Vec<f64>,
    snapshot_water: Vec<f64>,
    snapshot_time: f64,
    snapshot_accounting: WaterAccounting,
}

impl WaterWorkspace {
    pub fn new(node_count: usize) -> Result<Self, WaterStepError> {
        if node_count < 3 {
            return Err(WaterStepError::InvalidWorkspace {
                reason: "node count must be at least three",
            });
        }
        Ok(Self {
            lower: vec![0.0; node_count],
            diagonal: vec![0.0; node_count],
            upper: vec![0.0; node_count],
            rhs: vec![0.0; node_count],
            trial_heads: vec![0.0; node_count],
            trial_water: vec![0.0; node_count],
            snapshot_heads: vec![0.0; node_count],
            snapshot_water: vec![0.0; node_count],
            snapshot_time: 0.0,
            snapshot_accounting: WaterAccounting::default(),
        })
    }

    pub fn node_count(&self) -> usize {
        self.trial_heads.len()
    }

    pub(crate) fn snapshot(&mut self, state: &WaterState) {
        self.snapshot_heads.copy_from_slice(&state.heads);
        self.snapshot_water.copy_from_slice(&state.water_contents);
        self.snapshot_time = state.time;
        self.snapshot_accounting = state.accounting;
        self.trial_heads.copy_from_slice(&state.heads);
        self.trial_water.copy_from_slice(&state.water_contents);
    }

    pub(crate) fn restore(&mut self, state: &mut WaterState) {
        state.heads.copy_from_slice(&self.snapshot_heads);
        state.water_contents.copy_from_slice(&self.snapshot_water);
        state.time = self.snapshot_time;
        state.accounting = self.snapshot_accounting;
        self.trial_heads.copy_from_slice(&self.snapshot_heads);
        self.trial_water.copy_from_slice(&self.snapshot_water);
    }
}

pub(crate) fn validate_finite(name: &'static str, values: &[f64]) -> Result<(), WaterStepError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(WaterStepError::NonFiniteInput { name });
    }
    Ok(())
}
