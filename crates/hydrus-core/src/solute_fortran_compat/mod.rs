//! Private foundation for the pinned-Fortran-compatible solute backend.
#[allow(dead_code)] // wired by 18.12.1
mod adapter;
#[allow(dead_code)] // wired by 18.12.1
mod types;

#[allow(dead_code)] // wired by 18.10.1
mod accounting;
#[allow(dead_code)] // wired by 18.10.1
mod assembly;
mod biology;
#[allow(dead_code)] // wired by 18.10.1
mod boundary;
mod deposition;
mod driver;
#[allow(dead_code)] // wired by 18.10.1
mod dual_domain;
mod iteration;
mod nonequilibrium;
mod processes;
#[allow(dead_code)] // wired by 18.10.1
mod source_coupling;
#[allow(dead_code)] // wired by 18.10.1
mod transport;
