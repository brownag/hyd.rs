use crate::hydraulics::{
    transform_boundary_row, BoundaryCondition, BoundaryRow, BoundaryRowLocation, FaceAveraging,
};
use crate::WaterStepError;

/// Physical lower-boundary selections supported by the transactional water solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LowerBoundaryMode {
    /// Prescribe the lower-node pressure head. `row` is the caller-supplied
    /// lower row and `flux` the measured signed exchange, positive into the domain.
    FixedHead {
        head: f64,
        row: BoundaryRow,
        flux: f64,
    },
    /// Prescribe signed water flux, positive into the domain.
    PrescribedFlux { flux: f64 },
    /// Apply gravity drainage, with positive flux defined into the domain.
    FreeDrainage,
    /// A source mode whose behavior is not implemented by this product.
    Unsupported(UnsupportedLowerBoundaryMode),
}

/// Lower-boundary modes deliberately rejected until supported by evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedLowerBoundaryMode {
    SeepageFace,
    WaterTable,
}

pub(crate) struct LowerFluxRowInput {
    pub face_conductivity: f64,
    pub capacity: f64,
    pub head: f64,
    pub theta: f64,
    pub old_theta: f64,
    pub sink: f64,
    pub spacing: f64,
    pub gravity: f64,
    pub timestep: f64,
}

pub(crate) fn lower_flux_row(input: LowerFluxRowInput) -> Result<BoundaryRow, WaterStepError> {
    let LowerFluxRowInput {
        face_conductivity,
        capacity,
        head,
        theta,
        old_theta,
        sink,
        spacing,
        gravity,
        timestep,
    } = input;
    let inputs = [
        face_conductivity,
        capacity,
        head,
        theta,
        old_theta,
        sink,
        spacing,
        gravity,
        timestep,
    ];
    if inputs.iter().any(|value| !value.is_finite())
        || face_conductivity <= 0.0
        || capacity < 0.0
        || spacing <= 0.0
        || timestep <= 0.0
    {
        return Err(WaterStepError::InvalidLowerBoundary {
            name: "lower_flux_row_inputs",
        });
    }
    let half_width = spacing / 2.0;
    let conductance = face_conductivity / spacing;
    let storage = capacity * half_width / timestep;
    let row = BoundaryRow {
        lower: 0.0,
        diagonal: conductance + storage,
        upper: -conductance,
        rhs: storage * head - (theta - old_theta) * half_width / timestep
            + face_conductivity * gravity
            - sink * half_width,
    };
    if [row.lower, row.diagonal, row.upper, row.rhs]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(WaterStepError::InvalidLowerBoundary {
            name: "lower_flux_row_result",
        });
    }
    Ok(row)
}

impl LowerBoundaryMode {
    /// Validates the mode against the mesh and returns it for per-trial use.
    pub(crate) fn prepare(self, count: usize, minimum_pivot: f64) -> Result<Self, WaterStepError> {
        match self {
            Self::FixedHead { head, .. } if !head.is_finite() => {
                return Err(WaterStepError::InvalidLowerBoundary { name: "head" });
            }
            Self::PrescribedFlux { flux } if !flux.is_finite() => {
                return Err(WaterStepError::InvalidLowerBoundary { name: "flux" });
            }
            Self::Unsupported(UnsupportedLowerBoundaryMode::SeepageFace) => {
                return Err(WaterStepError::UnsupportedLowerBoundary {
                    mode: "seepage_face",
                });
            }
            Self::Unsupported(UnsupportedLowerBoundaryMode::WaterTable) => {
                return Err(WaterStepError::UnsupportedLowerBoundary {
                    mode: "water_table",
                });
            }
            Self::FixedHead { head, row, .. } => {
                transform_boundary_row(
                    count,
                    BoundaryRowLocation::Lower,
                    row,
                    BoundaryCondition::Dirichlet { value: head },
                    minimum_pivot,
                )?;
            }
            Self::FreeDrainage if count < 2 => {
                return Err(WaterStepError::InvalidLowerBoundary {
                    name: "free_drainage_mesh",
                });
            }
            Self::PrescribedFlux { .. } | Self::FreeDrainage => {}
        }
        Ok(self)
    }

    pub(crate) fn fixes_head(self) -> bool {
        matches!(self, Self::FixedHead { .. })
    }

    /// Signed measured exchange that must be finite before a step starts.
    pub(crate) fn measured_flux(self) -> Option<f64> {
        match self {
            Self::FixedHead { flux, .. } => Some(flux),
            _ => None,
        }
    }

    /// Builds the lower condition, exchange flux and matrix row for one trial.
    pub(crate) fn prepare_step(
        self,
        trial: LowerTrial,
    ) -> Result<PreparedLowerBoundary, WaterStepError> {
        let (condition, flux, caller_row) = match self {
            Self::FixedHead { head, row, flux } => (
                BoundaryCondition::Dirichlet { value: head },
                flux,
                Some(row),
            ),
            Self::PrescribedFlux { flux } => (BoundaryCondition::Neumann { flux }, flux, None),
            Self::FreeDrainage => {
                let conductivity_a = trial.conductivity_a;
                let conductivity_b = trial.conductivity_b;
                if !conductivity_a.is_finite()
                    || !conductivity_b.is_finite()
                    || conductivity_a <= 0.0
                    || conductivity_b <= 0.0
                    || !trial.gravity.is_finite()
                {
                    return Err(WaterStepError::InvalidLowerBoundary {
                        name: "free_drainage_properties",
                    });
                }
                let conductivity = match trial.averaging {
                    FaceAveraging::Arithmetic => (conductivity_a + conductivity_b) / 2.0,
                    FaceAveraging::Geometric => (conductivity_a * conductivity_b).sqrt(),
                };
                // WATFLOW.FOR Reset adds rBot=-ConB*Grav to its lower RHS.
                // Rust defines positive boundary flux as entering the domain,
                // so the source-derived free-drainage exchange is -K_face*g.
                let flux = -conductivity * trial.gravity;
                (BoundaryCondition::Neumann { flux }, flux, None)
            }
            Self::Unsupported(_) => {
                return Err(WaterStepError::InvalidLowerBoundary { name: "mode" });
            }
        };
        if !flux.is_finite() {
            return Err(WaterStepError::InvalidLowerBoundary { name: "flux" });
        }
        let row = match caller_row {
            Some(row) => row,
            None => trial.flux_row()?,
        };
        Ok(PreparedLowerBoundary {
            condition,
            exchange_flux: flux,
            row,
        })
    }
}

/// Trial-state values the lower endpoint needs to build its own row.
pub(crate) struct LowerTrial {
    pub conductivity_a: f64,
    pub conductivity_b: f64,
    pub capacity: f64,
    pub head: f64,
    pub theta: f64,
    pub old_theta: f64,
    pub sink: f64,
    pub spacing: f64,
    pub gravity: f64,
    pub timestep: f64,
    pub averaging: FaceAveraging,
}

impl LowerTrial {
    fn flux_row(&self) -> Result<BoundaryRow, WaterStepError> {
        let face_conductivity = match self.averaging {
            FaceAveraging::Arithmetic => (self.conductivity_a + self.conductivity_b) / 2.0,
            FaceAveraging::Geometric => (self.conductivity_a * self.conductivity_b).sqrt(),
        };
        lower_flux_row(LowerFluxRowInput {
            face_conductivity,
            capacity: self.capacity,
            head: self.head,
            theta: self.theta,
            old_theta: self.old_theta,
            sink: self.sink,
            spacing: self.spacing,
            gravity: self.gravity,
            timestep: self.timestep,
        })
    }
}

pub(crate) struct PreparedLowerBoundary {
    pub condition: BoundaryCondition,
    pub exchange_flux: f64,
    pub row: BoundaryRow,
}
