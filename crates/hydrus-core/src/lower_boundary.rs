use crate::hydraulics::{BoundaryCondition, BoundaryRow, FaceAveraging};
use crate::WaterStepError;

/// Physical lower-boundary selections supported by the transactional water solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LowerBoundaryMode {
    /// Prescribe the lower-node pressure head.
    FixedHead { head: f64 },
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
    pub(crate) fn validate(self) -> Result<(), WaterStepError> {
        match self {
            Self::FixedHead { head } if !head.is_finite() => {
                Err(WaterStepError::InvalidLowerBoundary { name: "head" })
            }
            Self::PrescribedFlux { flux } if !flux.is_finite() => {
                Err(WaterStepError::InvalidLowerBoundary { name: "flux" })
            }
            Self::Unsupported(UnsupportedLowerBoundaryMode::SeepageFace) => {
                Err(WaterStepError::UnsupportedLowerBoundary {
                    mode: "seepage_face",
                })
            }
            Self::Unsupported(UnsupportedLowerBoundaryMode::WaterTable) => {
                Err(WaterStepError::UnsupportedLowerBoundary {
                    mode: "water_table",
                })
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn condition(self, flux: f64) -> Result<BoundaryCondition, WaterStepError> {
        self.validate()?;
        match self {
            Self::FixedHead { head } => Ok(BoundaryCondition::Dirichlet { value: head }),
            Self::PrescribedFlux { flux } => Ok(BoundaryCondition::Neumann { flux }),
            Self::FreeDrainage => Ok(BoundaryCondition::Neumann { flux }),
            Self::Unsupported(_) => unreachable!("validated above"),
        }
    }

    pub(crate) fn flux_into_domain(
        self,
        conductivity_a: f64,
        conductivity_b: f64,
        gravity: f64,
        averaging: FaceAveraging,
        fixed_head_flux: f64,
    ) -> Result<f64, WaterStepError> {
        self.validate()?;
        let flux = match self {
            Self::FixedHead { .. } => fixed_head_flux,
            Self::PrescribedFlux { flux } => flux,
            Self::FreeDrainage => {
                if !conductivity_a.is_finite()
                    || !conductivity_b.is_finite()
                    || conductivity_a <= 0.0
                    || conductivity_b <= 0.0
                    || !gravity.is_finite()
                {
                    return Err(WaterStepError::InvalidLowerBoundary {
                        name: "free_drainage_properties",
                    });
                }
                let conductivity = match averaging {
                    FaceAveraging::Arithmetic => (conductivity_a + conductivity_b) / 2.0,
                    FaceAveraging::Geometric => (conductivity_a * conductivity_b).sqrt(),
                };
                // WATFLOW.FOR Reset adds rBot=-ConB*Grav to its lower RHS.
                // Rust defines positive boundary flux as entering the domain,
                // so the source-derived free-drainage exchange is -K_face*g.
                -conductivity * gravity
            }
            Self::Unsupported(_) => unreachable!("validated above"),
        };
        if !flux.is_finite() {
            return Err(WaterStepError::InvalidLowerBoundary { name: "flux" });
        }
        Ok(flux)
    }
}
