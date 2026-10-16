//! Private foundation for the pinned-Fortran-compatible solute backend.
#[allow(dead_code)] // wired by 18.12.1
mod adapter;
#[allow(dead_code)] // wired by 18.12.1
mod types;

mod accounting;
mod assembly;
mod biology;
#[allow(dead_code)] // canonical_endpoint is wired by 18.12.1
mod boundary;
#[allow(dead_code)] // deposit_nodes is wired by 18.12.1
mod deposition;
#[allow(dead_code)] // wired by 18.12.1
pub(crate) mod driver;
#[allow(dead_code)] // DualPor (WATFLOW) and dual_material_selection are wired by 18.12.1
mod dual_domain;
#[allow(dead_code)] // HYDRUS.FOR:279-288 initializers, wired by 18.12.1
mod initialization;
pub(crate) mod iteration;
mod nonequilibrium;
mod processes;
#[allow(dead_code)] // wired by 18.12.1
mod source_coupling;
mod transport;
#[allow(dead_code)] // wired by 20.4.1
pub(crate) mod water_compat;
