use crate::HydraulicError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VanGenuchtenParameters {
    residual_water_content: f64,
    saturated_water_content: f64,
    alpha: f64,
    n: f64,
    saturated_conductivity: f64,
    pore_connectivity: f64,
}

/// Mualem pore-connectivity exponent `l` (MATERIAL.FOR `BPar`, default 0.5).
const DEFAULT_PORE_CONNECTIVITY: f64 = 0.5;

impl VanGenuchtenParameters {
    pub fn new(
        residual_water_content: f64,
        saturated_water_content: f64,
        alpha: f64,
        n: f64,
        saturated_conductivity: f64,
    ) -> Result<Self, HydraulicError> {
        let parameters = Self {
            residual_water_content,
            saturated_water_content,
            alpha,
            n,
            saturated_conductivity,
            pore_connectivity: DEFAULT_PORE_CONNECTIVITY,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    /// Returns a copy with pore-connectivity exponent `l`, validated in `[0, 10]`.
    pub fn with_pore_connectivity(self, pore_connectivity: f64) -> Result<Self, HydraulicError> {
        let parameters = Self {
            pore_connectivity,
            ..self
        };
        parameters.validate()?;
        Ok(parameters)
    }

    pub fn validate(&self) -> Result<(), HydraulicError> {
        for (name, value) in [
            ("residual_water_content", self.residual_water_content),
            ("saturated_water_content", self.saturated_water_content),
            ("alpha", self.alpha),
            ("n", self.n),
            ("saturated_conductivity", self.saturated_conductivity),
            ("pore_connectivity", self.pore_connectivity),
        ] {
            if !value.is_finite() {
                return Err(HydraulicError::NonFiniteParameter { name });
            }
        }
        if self.residual_water_content >= self.saturated_water_content {
            return Err(HydraulicError::InvalidParameter {
                name: "water_content",
                reason: "residual must be below saturated",
            });
        }
        if self.alpha <= 0.0 {
            return Err(HydraulicError::InvalidParameter {
                name: "alpha",
                reason: "must be positive",
            });
        }
        if self.n <= 1.0 {
            return Err(HydraulicError::InvalidParameter {
                name: "n",
                reason: "must be greater than one",
            });
        }
        if self.saturated_conductivity < 0.0 {
            return Err(HydraulicError::InvalidParameter {
                name: "saturated_conductivity",
                reason: "must be non-negative",
            });
        }
        if !(0.0..=10.0).contains(&self.pore_connectivity) {
            return Err(HydraulicError::InvalidParameter {
                name: "pore_connectivity",
                reason: "must be in [0, 10]",
            });
        }
        Ok(())
    }

    pub fn residual_water_content(&self) -> f64 {
        self.residual_water_content
    }

    pub fn saturated_water_content(&self) -> f64 {
        self.saturated_water_content
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    pub fn n(&self) -> f64 {
        self.n
    }

    pub fn saturated_conductivity(&self) -> f64 {
        self.saturated_conductivity
    }

    pub fn pore_connectivity(&self) -> f64 {
        self.pore_connectivity
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Evaluation {
    pub theta: f64,
    pub conductivity: f64,
    pub capacity: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HydraulicDerivatives {
    pub dtheta_dhead: f64,
    pub dconductivity_dhead: f64,
    pub dcapacity_dhead: f64,
}

/// Validated one-dimensional profile data used by later geometry and stencil
/// contracts. Indices exposed by this type are always zero-based.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh1D {
    coordinates: Vec<f64>,
    material_indices: Vec<usize>,
    layer_indices: Option<Vec<usize>>,
    material_count: usize,
    layer_count: usize,
    inclination_cosine: f64,
}

impl Mesh1D {
    /// Constructs a mesh from already zero-based indices.
    pub fn new(
        coordinates: Vec<f64>,
        material_indices: Vec<usize>,
        layer_indices: Option<Vec<usize>>,
        material_count: usize,
        layer_count: usize,
        inclination_cosine: f64,
    ) -> Result<Self, HydraulicError> {
        let mesh = Self {
            coordinates,
            material_indices,
            layer_indices,
            material_count,
            layer_count,
            inclination_cosine,
        };
        mesh.validate()?;
        Ok(mesh)
    }

    /// Converts one-based source profile indices into the zero-based contract.
    pub fn from_source_one_based(
        coordinates: Vec<f64>,
        material_indices: &[usize],
        layer_indices: Option<&[usize]>,
        material_count: usize,
        layer_count: usize,
        inclination_cosine: f64,
    ) -> Result<Self, HydraulicError> {
        let materials =
            convert_source_indices(material_indices, material_count, "material_indices")?;
        let layers = layer_indices
            .map(|indices| convert_source_indices(indices, layer_count, "layer_indices"))
            .transpose()?;
        Self::new(
            coordinates,
            materials,
            layers,
            material_count,
            layer_count,
            inclination_cosine,
        )
    }

    pub fn coordinates(&self) -> &[f64] {
        &self.coordinates
    }

    pub fn material_indices(&self) -> &[usize] {
        &self.material_indices
    }

    pub fn layer_indices(&self) -> Option<&[usize]> {
        self.layer_indices.as_deref()
    }

    pub fn material_count(&self) -> usize {
        self.material_count
    }

    pub fn layer_count(&self) -> usize {
        self.layer_count
    }

    pub fn inclination_cosine(&self) -> f64 {
        self.inclination_cosine
    }

    fn validate(&self) -> Result<(), HydraulicError> {
        if self.coordinates.is_empty() {
            return Err(mesh_error("coordinates", "must not be empty"));
        }
        if self.coordinates.len() != self.material_indices.len() {
            return Err(mesh_error(
                "material_indices",
                "length must match coordinates",
            ));
        }
        if let Some(layers) = &self.layer_indices {
            if layers.len() != self.coordinates.len() {
                return Err(mesh_error("layer_indices", "length must match coordinates"));
            }
            if self.layer_count == 0 {
                return Err(mesh_error(
                    "layer_count",
                    "must be positive when layers exist",
                ));
            }
        }
        if self.material_count == 0 {
            return Err(mesh_error("material_count", "must be positive"));
        }
        for coordinate in &self.coordinates {
            if !coordinate.is_finite() {
                return Err(mesh_error("coordinates", "must be finite"));
            }
        }
        for pair in self.coordinates.windows(2) {
            if pair[1] <= pair[0] {
                return Err(mesh_error("coordinates", "must be strictly increasing"));
            }
        }
        validate_indices(
            &self.material_indices,
            self.material_count,
            "material_indices",
        )?;
        if let Some(layers) = &self.layer_indices {
            validate_indices(layers, self.layer_count, "layer_indices")?;
        }
        if !self.inclination_cosine.is_finite() {
            return Err(mesh_error("inclination_cosine", "must be finite"));
        }
        Ok(())
    }
}

fn convert_source_indices(
    indices: &[usize],
    count: usize,
    name: &'static str,
) -> Result<Vec<usize>, HydraulicError> {
    indices
        .iter()
        .map(|&index| {
            if index == 0 {
                return Err(mesh_error(name, "source indices must be one-based"));
            }
            if index > count {
                return Err(mesh_error(name, "source index is out of range"));
            }
            Ok(index - 1)
        })
        .collect()
}

fn validate_indices(
    indices: &[usize],
    count: usize,
    name: &'static str,
) -> Result<(), HydraulicError> {
    if indices.iter().any(|&index| index >= count) {
        return Err(mesh_error(name, "zero-based index is out of range"));
    }
    Ok(())
}

fn mesh_error(name: &'static str, reason: &'static str) -> HydraulicError {
    HydraulicError::InvalidMesh { name, reason }
}
