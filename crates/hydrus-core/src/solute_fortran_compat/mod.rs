//! Private foundation for the pinned-Fortran-compatible solute backend.
mod adapter;
mod types;

mod accounting;
mod assembly;
mod biology;
mod boundary;
pub(crate) mod compat_runner;
pub(crate) mod deck_reader;
mod deposition;
pub(crate) mod driver;
mod dual_domain;
mod initialization;
pub(crate) mod iteration;
mod nonequilibrium;
pub(crate) mod output_compat;
mod processes;
mod source_coupling;
mod transport;
pub(crate) mod water_compat;
