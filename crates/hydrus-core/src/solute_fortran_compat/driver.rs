//! Source-ordered `Solute` step driver (`SOLUTE.FOR:6-240`), task 18.10.1.
//!
//! The driver composes the accepted kernels in the order the source calls them. Per
//! species `jS`: the prologue (36-92), `SetSSnk` (94-96), then one iteration pass
//! (label 12): for each `Level` `Coeff` -> `MatSet` -> `vCorr` restore (14) ->
//! beginning `MassTran` (`Level==1 && Iter==1`, 135-140); then `BanSol`, the update and
//! convergence test (16); and, once the pass is accepted, label 17, `SorbConc`, the end
//! `MassTran`, and the flux closure (188-225). `FluxConc` follows the species loop.
//!
//! `Exclusion` is *not* called from `Solute`: `HYDRUS.FOR:390-391`
//! (`if(lBact.and..not.lWat) call Exclusion(...)`) runs it before the solute call, so
//! [`solute_step`] performs it first, before the species loop.
//!
//! # Nonlinear seam for 18.11.1
//!
//! [`solve_pass`] is exactly one execution of labels 12-16 and reports `converged`.
//! The retry logic (`goto 12`, the `dt/3` restart at label 10 and `ierr`) belongs to
//! 18.11.1: when a nonlinear species does not converge, [`solute_step`] stops and
//! returns it as `pending`; the caller continues with [`solve_pass`] /
//! [`finish_species`] / [`solute_step_from`].
//!
//! # State mapping
//!
//! * `Coeff` locals that survive between calls are explicit state: `cMid`
//!   (assigned only in the nonlinear branch, 573, yet read at 687 and by `Disper`;
//!   `None` is an error for a linear species, never `0`), `Dc`/`aa`
//!   ([`CarriedLocals`]) and `DMobI`.
//! * `ThOIm` (after `Update`, `WATFLOW.FOR:1112`), `ThNIm` and `SinkIm` come from the
//!   caller. `DualPor` itself is a `WATFLOW` call and is not made here.
//! * The endpoint kernel evaluates both `MatSet` endpoint levels together from the
//!   per-level snapshots; it is recorded as part of the `Level==2` `MatSet`.
//! * `lBact` with a chain, and `lBact` with `iDualPor>0`, are rejected.

use super::accounting::{flux_concentration, FluxConcInput, MassTotals, MassTranInput};
use super::assembly::{assemble_interior_into, ban_solve, AssemblyInputs, AssemblyLevel};
use super::biology::{
    select_new_level, select_old_level, BacteriaNode, BacteriaScalars, BacteriaSpec, CarriedLocals,
};
use super::boundary::assemble_endpoints;
use super::deposition::{blocking, deposit, DepositInput};
use super::dual_domain::{dual_domain_th_imob, dual_neighbor_theta};
use super::nonequilibrium::{
    immobile_water, nequil, omega_terms, should_call_nequil, ImmobileWaterSpec, NEquilFlags,
    NEquilInput, NEquilPools, OmegaSpec,
};
use super::processes::{
    chain_source, coeff_factors, isotherm_factors, old_level_factors, prev_species_factors,
    prev_species_isotherm, should_call_sorb_conc, sorb_conc, source_terms, ChainSourceInput,
    IsothermInput, MoistureFactorTable, SorbConcSpec, SorbConcState, SourceTermsInput,
};
use super::source_coupling::root_solute_uptake;
#[cfg(test)]
use super::source_coupling::{size_exclusion, SizeExclusion};
use super::transport::{coefficients, pecour, PecletInputs, TransportNode};
use super::types::{
    DefaultReal, DoubleReal, EndpointPrimitiveState, Explicit, KernelError,
    MaterialSpeciesParameters, RootUptakePrimitiveState, SizeExclusionPrimitiveState,
};

const TR: f32 = 293.15;
const GAS_R: f32 = 8.314;
const R_MIN: f32 = 1.0e-30;

/// `TT=(Temp+273.15-Tr)/R/(Temp+273.15)/Tr`.
fn tfac(t: f32) -> f32 {
    (t + 273.15 - TR) / GAS_R / (t + 273.15) / TR
}

fn dr(v: &[f32]) -> Vec<DefaultReal> {
    v.iter().map(|&x| DefaultReal(x)).collect()
}

/// Everything `Solute` receives that it never changes.
#[derive(Clone, Debug)]
pub(crate) struct SoluteStepConfig {
    /// `x`, native node order.
    pub x: Vec<f32>,
    /// `MatNum`, one-based.
    pub mat_num: Vec<usize>,
    /// `thSat(M)`.
    pub th_sat: Vec<f32>,
    /// `lMobIm(M)`.
    pub mobile_immobile: Vec<bool>,
    /// `ChPar(slot, M)` as `ch_par[M][slot-1]`; `ChPar(4,1)` is mutated by `Exclusion`.
    pub ch_par: Vec<Vec<f32>>,
    pub t_dep: Vec<f32>,
    pub species_count: usize,
    /// `lLinear(jS)`.
    pub linear: Vec<bool>,
    pub equil: bool,
    pub upwind: bool,
    pub art_disp: bool,
    pub tort: bool,
    pub i_tort: i32,
    pub bact: bool,
    pub filtr: bool,
    pub dual_neq: bool,
    /// `lWat`.
    pub water_flow: bool,
    /// `SinkF`.
    pub root_sink: bool,
    pub vapor: bool,
    pub atm_bc: bool,
    pub density: bool,
    /// `iDualPor`.
    pub dual_por: u8,
    /// `iConcType`.
    pub conc_type: i32,
    pub moisture_selector: u8,
    pub moisture: Option<MoistureFactorTable>,
    pub epsi: f32,
    pub t_pulse: f32,
    pub dt_min: f32,
    pub k_bot: i32,
    pub k_top: i32,
    pub c_atm: f32,
    pub d_surf: f32,
    /// `rBot`.
    pub r_bot: f32,
    pub t_level: i32,
    /// `PeCr`.
    pub pe_cr: f32,
    pub x_conv: f32,
    pub t_conv: f32,
    pub c_tol_a: f32,
    pub c_tol_r: f32,
    /// `SetSSnk` actuals other than `Conc(jS,.)`, `jS` and `t` (set per species).
    pub root_uptake: Option<RootUptakePrimitiveState>,
    /// `Exclusion` actuals other than `ThNew(1)`/`vNew(1)` (taken from the old level).
    pub size_exclusion: Option<SizeExclusionPrimitiveState>,
}

/// The mutable arrays and scalars of one `Solute` call.
#[derive(Clone, Debug)]
pub(crate) struct SoluteStepState {
    /// `Conc(jS,i)`, `Sorb(jS,i)`, `Sorb2(jS,i)` as `[species][node]`.
    pub conc: Vec<Vec<f32>>,
    pub sorb: Vec<Vec<f32>>,
    pub sorb2: Vec<Vec<f32>>,
    pub c_new: Vec<f32>,
    pub sorb_n: Vec<f32>,
    pub sorb_n2: Vec<f32>,
    pub c_prev_o: Vec<f32>,
    pub c_temp: Vec<f32>,
    pub th_o: Vec<f32>,
    pub th_n: Vec<f32>,
    pub v_o: Vec<f32>,
    pub v_n: Vec<f32>,
    pub v_corr: Vec<f32>,
    pub temp_o: Vec<f32>,
    pub temp_n: Vec<f32>,
    pub th_o_im: Vec<f32>,
    pub th_n_im: Vec<f32>,
    pub sink_im: Vec<f32>,
    pub s_trans: Vec<f32>,
    /// `sSink`.
    pub s_sink: Vec<f32>,
    pub wc: Vec<f32>,
    pub q0: Vec<f32>,
    pub q1: Vec<f32>,
    /// `cTop(jS)`, `cBot(jS)` (reset by the pulse test).
    pub c_top: Vec<f32>,
    pub c_bot: Vec<f32>,
    pub cv_top: Vec<f32>,
    pub cv_bot: Vec<f32>,
    pub totals: MassTotals,
    pub carried: CarriedLocals,
    /// `cMid`; `None` until some `Coeff` call assigns it.
    pub c_mid: Option<f32>,
    pub dmob_i: f32,
    /// `t` (double) and `dt`.
    pub t: f64,
    pub dt: f32,
    pub peclet: f32,
    pub courant: f32,
    pub dt_max_c: f32,
    pub iter_c: u32,
    pub ierr: i32,
    /// Per-species data that survives from the prologue to the flux closure.
    pub frame: Option<SpeciesFrame>,
    /// Source call order, for tests and diagnostics.
    pub trace: Vec<DriverEvent>,
}

/// Prologue snapshots and the endpoint primitives of the last pass.
#[derive(Clone, Debug)]
pub(crate) struct SpeciesFrame {
    /// `vO` and `Conc(jS,.)` as the prologue (`64-72`) saw them.
    pub v_o0: Vec<f32>,
    pub conc0: Vec<f32>,
    pub endpoint: Option<EndpointPrimitiveState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DriverEvent {
    #[cfg(test)]
    Exclusion,
    SetSSnk(usize),
    Coeff(usize, u8),
    MatSet(usize, u8),
    MassTranBegin(usize),
    BanSol(usize),
    Update(usize),
    SorbConc(usize),
    MassTranEnd(usize),
    FluxClose(usize),
    FluxConc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PassOutcome {
    pub converged: bool,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SoluteStepOutcome {
    /// A nonlinear species whose pass did not converge; 18.11.1 continues from it.
    pub pending: Option<usize>,
}

impl SoluteStepState {
    /// Zero-filled state of `nodes` nodes and `species` species (callers overwrite).
    pub(crate) fn blank(nodes: usize, species: usize) -> Self {
        let z = || vec![0.0_f32; nodes];
        let per = || vec![vec![0.0_f32; nodes]; species];
        Self {
            conc: per(),
            sorb: per(),
            sorb2: per(),
            c_new: z(),
            sorb_n: z(),
            sorb_n2: z(),
            c_prev_o: z(),
            c_temp: z(),
            th_o: z(),
            th_n: z(),
            v_o: z(),
            v_n: z(),
            v_corr: z(),
            temp_o: z(),
            temp_n: z(),
            th_o_im: z(),
            th_n_im: z(),
            sink_im: z(),
            s_trans: z(),
            s_sink: z(),
            wc: z(),
            q0: z(),
            q1: z(),
            c_top: vec![0.0; species],
            c_bot: vec![0.0; species],
            cv_top: vec![0.0; species],
            cv_bot: vec![0.0; species],
            totals: MassTotals {
                cv_ch0: vec![DefaultReal(0.0); species],
                cv_ch1: vec![DefaultReal(0.0); species],
                cv_ch_r: vec![DefaultReal(0.0); species],
                cv_ch_im: vec![DefaultReal(0.0); species],
                s_trans: vec![DefaultReal(0.0); nodes],
            },
            carried: CarriedLocals::default(),
            c_mid: None,
            dmob_i: f32::NAN,
            t: 0.0,
            dt: 0.0,
            peclet: 0.0,
            courant: 0.0,
            dt_max_c: 1.0e30,
            iter_c: 1,
            ierr: 0,
            frame: Some(SpeciesFrame {
                v_o0: z(),
                conc0: z(),
                endpoint: Some(blank_endpoint(nodes, 16)),
            }),
            trace: Vec::with_capacity(64),
        }
    }

    /// Zero-filled state with specified material capacity for endpoint buffers.
    pub(crate) fn blank_with_materials(nodes: usize, species: usize, materials: usize) -> Self {
        let mut s = Self::blank(nodes, species);
        if let Some(ref mut frame) = s.frame {
            frame.endpoint = Some(blank_endpoint(nodes, materials));
        }
        s
    }

    /// Copy all data from `other` into `self`, reusing existing `Vec` capacities without reallocating.
    pub(crate) fn copy_from(&mut self, other: &Self) {
        fn copy_vec_f32(dst: &mut Vec<f32>, src: &[f32]) {
            if dst.len() == src.len() {
                dst.copy_from_slice(src);
            } else {
                dst.clear();
                dst.extend_from_slice(src);
            }
        }
        fn copy_vec_dr(dst: &mut Vec<DefaultReal>, src: &[DefaultReal]) {
            if dst.len() == src.len() {
                dst.copy_from_slice(src);
            } else {
                dst.clear();
                dst.extend_from_slice(src);
            }
        }
        fn copy_2d_f32(dst: &mut Vec<Vec<f32>>, src: &[Vec<f32>]) {
            if dst.len() == src.len() {
                for (d, s) in dst.iter_mut().zip(src) {
                    copy_vec_f32(d, s);
                }
            } else {
                dst.clear();
                dst.extend(src.iter().cloned());
            }
        }

        copy_2d_f32(&mut self.conc, &other.conc);
        copy_2d_f32(&mut self.sorb, &other.sorb);
        copy_2d_f32(&mut self.sorb2, &other.sorb2);

        copy_vec_f32(&mut self.c_new, &other.c_new);
        copy_vec_f32(&mut self.sorb_n, &other.sorb_n);
        copy_vec_f32(&mut self.sorb_n2, &other.sorb_n2);
        copy_vec_f32(&mut self.c_prev_o, &other.c_prev_o);
        copy_vec_f32(&mut self.c_temp, &other.c_temp);
        copy_vec_f32(&mut self.th_o, &other.th_o);
        copy_vec_f32(&mut self.th_n, &other.th_n);
        copy_vec_f32(&mut self.v_o, &other.v_o);
        copy_vec_f32(&mut self.v_n, &other.v_n);
        copy_vec_f32(&mut self.v_corr, &other.v_corr);
        copy_vec_f32(&mut self.temp_o, &other.temp_o);
        copy_vec_f32(&mut self.temp_n, &other.temp_n);
        copy_vec_f32(&mut self.th_o_im, &other.th_o_im);
        copy_vec_f32(&mut self.th_n_im, &other.th_n_im);
        copy_vec_f32(&mut self.sink_im, &other.sink_im);
        copy_vec_f32(&mut self.s_trans, &other.s_trans);
        copy_vec_f32(&mut self.s_sink, &other.s_sink);
        copy_vec_f32(&mut self.wc, &other.wc);
        copy_vec_f32(&mut self.q0, &other.q0);
        copy_vec_f32(&mut self.q1, &other.q1);
        copy_vec_f32(&mut self.c_top, &other.c_top);
        copy_vec_f32(&mut self.c_bot, &other.c_bot);
        copy_vec_f32(&mut self.cv_top, &other.cv_top);
        copy_vec_f32(&mut self.cv_bot, &other.cv_bot);

        copy_vec_dr(&mut self.totals.cv_ch0, &other.totals.cv_ch0);
        copy_vec_dr(&mut self.totals.cv_ch1, &other.totals.cv_ch1);
        copy_vec_dr(&mut self.totals.cv_ch_r, &other.totals.cv_ch_r);
        copy_vec_dr(&mut self.totals.cv_ch_im, &other.totals.cv_ch_im);
        copy_vec_dr(&mut self.totals.s_trans, &other.totals.s_trans);

        self.carried = other.carried;
        self.c_mid = other.c_mid;
        self.dmob_i = other.dmob_i;
        self.t = other.t;
        self.dt = other.dt;
        self.peclet = other.peclet;
        self.courant = other.courant;
        self.dt_max_c = other.dt_max_c;
        self.iter_c = other.iter_c;
        self.ierr = other.ierr;
        match (&mut self.frame, &other.frame) {
            (Some(dst), Some(src)) => {
                copy_vec_f32(&mut dst.v_o0, &src.v_o0);
                copy_vec_f32(&mut dst.conc0, &src.conc0);
                match (&mut dst.endpoint, &src.endpoint) {
                    (Some(dst_ep), Some(src_ep)) => {
                        copy_endpoint_in_place(dst_ep, src_ep);
                    }
                    (None, Some(src_ep)) => {
                        dst.endpoint = Some(src_ep.clone());
                    }
                    _ => {}
                }
            }
            _ => self.frame = other.frame.clone(),
        }
        self.trace.clear();
        self.trace.extend_from_slice(&other.trace);
    }
}

pub(crate) fn blank_endpoint(nodes: usize, materials: usize) -> EndpointPrimitiveState {
    let n = nodes;
    let z_dr = || vec![DefaultReal(0.0); n];
    EndpointPrimitiveState {
        coordinates: z_dr(),
        old_water: z_dr(),
        new_water: z_dr(),
        old_velocity: z_dr(),
        new_velocity: z_dr(),
        old_matrix_velocity: z_dr(),
        new_matrix_velocity: z_dr(),
        old_dispersion: z_dr(),
        new_dispersion: z_dr(),
        old_retardation: z_dr(),
        new_retardation: z_dr(),
        old_concentration: z_dr(),
        new_accepted_concentration: z_dr(),
        material_immobile_water: vec![DefaultReal(0.0); materials],
        old_source_g0: z_dr(),
        old_source_g1: z_dr(),
        new_source_g0: z_dr(),
        new_source_g1: z_dr(),
        upwind_weight: vec![DefaultReal(0.0); n.saturating_sub(1)],
        epsi: DefaultReal(0.0),
        dt: DefaultReal(0.0),
        alf: DefaultReal(0.0),
        top_atmospheric_concentration: None,
        top_surface_distance: None,
        old_temperature: None,
        new_temperature: None,
        source_chpar: None,
        source_tdep: None,
        material_ids: vec![0; n],
        source_species: 1,
        mobile_immobile: vec![false; materials],
        bacteria: false,
        dual_porosity: 0,
        bottom_resistance: DefaultReal(0.0),
        bottom_prescribed: DefaultReal(0.0),
        top_prescribed: DefaultReal(0.0),
        t_level: 0,
        vapor: false,
        r_min: DefaultReal(R_MIN),
    }
}

pub(crate) fn copy_endpoint_in_place(
    dst: &mut EndpointPrimitiveState,
    src: &EndpointPrimitiveState,
) {
    fn copy_dr(dst: &mut Vec<DefaultReal>, src: &[DefaultReal]) {
        if dst.len() == src.len() {
            dst.copy_from_slice(src);
        } else {
            dst.clear();
            dst.extend_from_slice(src);
        }
    }
    copy_dr(&mut dst.coordinates, &src.coordinates);
    copy_dr(&mut dst.old_water, &src.old_water);
    copy_dr(&mut dst.new_water, &src.new_water);
    copy_dr(&mut dst.old_velocity, &src.old_velocity);
    copy_dr(&mut dst.new_velocity, &src.new_velocity);
    copy_dr(&mut dst.old_matrix_velocity, &src.old_matrix_velocity);
    copy_dr(&mut dst.new_matrix_velocity, &src.new_matrix_velocity);
    copy_dr(&mut dst.old_dispersion, &src.old_dispersion);
    copy_dr(&mut dst.new_dispersion, &src.new_dispersion);
    copy_dr(&mut dst.old_retardation, &src.old_retardation);
    copy_dr(&mut dst.new_retardation, &src.new_retardation);
    copy_dr(&mut dst.old_concentration, &src.old_concentration);
    copy_dr(
        &mut dst.new_accepted_concentration,
        &src.new_accepted_concentration,
    );
    copy_dr(
        &mut dst.material_immobile_water,
        &src.material_immobile_water,
    );
    copy_dr(&mut dst.old_source_g0, &src.old_source_g0);
    copy_dr(&mut dst.old_source_g1, &src.old_source_g1);
    copy_dr(&mut dst.new_source_g0, &src.new_source_g0);
    copy_dr(&mut dst.new_source_g1, &src.new_source_g1);
    copy_dr(&mut dst.upwind_weight, &src.upwind_weight);
    dst.epsi = src.epsi;
    dst.dt = src.dt;
    dst.alf = src.alf;
    dst.top_atmospheric_concentration = src.top_atmospheric_concentration;
    dst.top_surface_distance = src.top_surface_distance;
    dst.old_temperature = src.old_temperature;
    dst.new_temperature = src.new_temperature;
    dst.source_chpar = src.source_chpar.clone();
    dst.source_tdep = src.source_tdep.clone();
    if dst.material_ids.len() == src.material_ids.len() {
        dst.material_ids.copy_from_slice(&src.material_ids);
    } else {
        dst.material_ids.clear();
        dst.material_ids.extend_from_slice(&src.material_ids);
    }
    dst.source_species = src.source_species;
    if dst.mobile_immobile.len() == src.mobile_immobile.len() {
        dst.mobile_immobile.copy_from_slice(&src.mobile_immobile);
    } else {
        dst.mobile_immobile.clear();
        dst.mobile_immobile.extend_from_slice(&src.mobile_immobile);
    }
    dst.bacteria = src.bacteria;
    dst.dual_porosity = src.dual_porosity;
    dst.bottom_resistance = src.bottom_resistance;
    dst.bottom_prescribed = src.bottom_prescribed;
    dst.top_prescribed = src.top_prescribed;
    dst.t_level = src.t_level;
    dst.vapor = src.vapor;
    dst.r_min = src.r_min;
}

/// Reusable preallocated working buffers for solute transport passes.
pub(crate) struct SoluteDriverScratch {
    pub(crate) backup_state: SoluteStepState,
    pub(crate) th_o_adj: Vec<f32>,
    pub(crate) th_n_adj: Vec<f32>,
    pub(crate) old_matrix_v: Vec<f32>,
    pub(crate) new_matrix_v: Vec<f32>,
    pub(crate) l1_disp: Vec<f32>,
    pub(crate) l1_retard: Vec<f32>,
    pub(crate) l1_g0: Vec<f32>,
    pub(crate) l1_g1: Vec<f32>,
    pub(crate) l2_disp: Vec<f32>,
    pub(crate) l2_retard: Vec<f32>,
    pub(crate) l2_g0: Vec<f32>,
    pub(crate) l2_g1: Vec<f32>,
    pub(crate) rows_old_rhs: Vec<f64>,
    pub(crate) lower: Vec<f64>,
    pub(crate) diag: Vec<f64>,
    pub(crate) upper: Vec<f64>,
    pub(crate) rhs: Vec<f64>,
    pub(crate) mass_bufs: MassBufs,
    pub(crate) endpoint: EndpointPrimitiveState,
    pub(crate) sorb2_scratch: Vec<f32>,
    pub(crate) deposited: Vec<(f32, f32)>,
}

impl SoluteDriverScratch {
    pub(crate) fn new(node_count: usize, species_count: usize, material_count: usize) -> Self {
        let n = node_count;
        let z = || vec![0.0_f32; n];
        let z64 = || vec![0.0_f64; n];
        let z_dr = || vec![DefaultReal(0.0); n];
        Self {
            backup_state: SoluteStepState::blank_with_materials(n, species_count, material_count),
            th_o_adj: z(),
            th_n_adj: z(),
            old_matrix_v: z(),
            new_matrix_v: z(),
            l1_disp: z(),
            l1_retard: z(),
            l1_g0: z(),
            l1_g1: z(),
            l2_disp: z(),
            l2_retard: z(),
            l2_g0: z(),
            l2_g1: z(),
            rows_old_rhs: z64(),
            lower: z64(),
            diag: z64(),
            upper: z64(),
            rhs: z64(),
            mass_bufs: MassBufs {
                x: z_dr(),
                mat0: vec![0; n],
                temp_o: z_dr(),
                temp_n: z_dr(),
                chpar: vec![vec![DefaultReal(0.0); species_count * 16 + 4]; material_count],
                tdep: vec![DefaultReal(0.0); species_count * 16 + 4],
                conc: z_dr(),
                sorb: z_dr(),
                sorb2: z_dr(),
                q0: z_dr(),
                q1: z_dr(),
                s_sink: z_dr(),
                th_o: z_dr(),
                th_n: z_dr(),
                v_o: z_dr(),
                v_n: z_dr(),
                sink_im: z_dr(),
            },
            endpoint: EndpointPrimitiveState {
                coordinates: z_dr(),
                old_water: z_dr(),
                new_water: z_dr(),
                old_velocity: z_dr(),
                new_velocity: z_dr(),
                old_matrix_velocity: z_dr(),
                new_matrix_velocity: z_dr(),
                old_dispersion: z_dr(),
                new_dispersion: z_dr(),
                old_retardation: z_dr(),
                new_retardation: z_dr(),
                old_concentration: z_dr(),
                new_accepted_concentration: z_dr(),
                material_immobile_water: vec![DefaultReal(0.0); material_count],
                old_source_g0: z_dr(),
                old_source_g1: z_dr(),
                new_source_g0: z_dr(),
                new_source_g1: z_dr(),
                upwind_weight: vec![DefaultReal(0.0); n.saturating_sub(1)],
                epsi: DefaultReal(0.0),
                dt: DefaultReal(0.0),
                alf: DefaultReal(0.0),
                top_atmospheric_concentration: None,
                top_surface_distance: None,
                old_temperature: None,
                new_temperature: None,
                source_chpar: None,
                source_tdep: None,
                material_ids: vec![0; n],
                source_species: 1,
                mobile_immobile: vec![false; material_count],
                bacteria: false,
                dual_porosity: 0,
                bottom_resistance: DefaultReal(0.0),
                bottom_prescribed: DefaultReal(0.0),
                top_prescribed: DefaultReal(0.0),
                t_level: 0,
                vapor: false,
                r_min: DefaultReal(R_MIN),
            },
            sorb2_scratch: z(),
            deposited: vec![(0.0, 0.0); n],
        }
    }
}

fn validate(cfg: &SoluteStepConfig, st: &SoluteStepState) -> Result<(), KernelError> {
    let n = cfg.x.len();
    let ns = cfg.species_count;
    if n < 3 || ns == 0 {
        return Err("the solute driver needs at least three nodes and one species");
    }
    if cfg.bact && ns > 1 {
        return Err("lBact with a species chain stops the source (SOLUTE.FOR:657-682)");
    }
    if cfg.bact && cfg.dual_por > 0 {
        return Err("lBact with iDualPor>0 is unsupported");
    }
    if cfg.dual_por > 2 || cfg.moisture_selector > 1 {
        return Err("iDualPor must be 0..2 and the moisture selector 0 or 1");
    }
    if cfg.moisture_selector == 1 && cfg.moisture.is_none() {
        return Err("moisture selector 1 needs the WDep table");
    }
    let nm = cfg.th_sat.len();
    if nm == 0
        || cfg.mobile_immobile.len() != nm
        || cfg.ch_par.len() != nm
        || cfg.ch_par.iter().any(|r| r.len() < ns * 16 + 4)
        || cfg.t_dep.len() < ns * 16 + 4
        || cfg.linear.len() != ns
        || cfg.mat_num.len() != n
        || cfg.mat_num.iter().any(|&m| m == 0 || m > nm)
    {
        return Err("material, ChPar, TDep or species arrays disagree with the dimensions");
    }
    if cfg.root_sink && cfg.root_uptake.is_none() {
        return Err("SinkF needs the SetSSnk primitive state");
    }
    if cfg.bact && !cfg.water_flow && cfg.size_exclusion.is_none() {
        return Err("lBact without lWat needs the Exclusion primitive state");
    }
    let node_arrays: [&Vec<f32>; 21] = [
        &st.c_new,
        &st.sorb_n,
        &st.sorb_n2,
        &st.c_prev_o,
        &st.c_temp,
        &st.th_o,
        &st.th_n,
        &st.v_o,
        &st.v_n,
        &st.v_corr,
        &st.temp_o,
        &st.temp_n,
        &st.th_o_im,
        &st.th_n_im,
        &st.sink_im,
        &st.s_trans,
        &st.s_sink,
        &st.wc,
        &st.q0,
        &st.q1,
        &cfg.x,
    ];
    let species_arrays = [&st.conc, &st.sorb, &st.sorb2];
    if node_arrays.iter().any(|a| a.len() != n)
        || species_arrays
            .iter()
            .any(|a| a.len() != ns || a.iter().any(|r| r.len() != n))
        || st.c_top.len() != ns
        || st.c_bot.len() != ns
        || st.cv_top.len() != ns
        || st.cv_bot.len() != ns
        || st.totals.cv_ch0.len() != ns
        || st.totals.s_trans.len() != n
    {
        return Err("state array lengths disagree with the node and species counts");
    }
    if !(0.0..=1.0).contains(&cfg.epsi) || st.dt <= 0.0 {
        return Err("epsi must lie in [0,1] and dt must be positive");
    }
    Ok(())
}

/// One full `Solute` call up to the first unconverged nonlinear species.
#[cfg(test)]
pub(crate) fn solute_step(
    cfg: &mut SoluteStepConfig,
    st: &mut SoluteStepState,
) -> Result<SoluteStepOutcome, KernelError> {
    let mut scratch = SoluteDriverScratch::new(cfg.x.len(), cfg.species_count, cfg.th_sat.len());
    solute_step_with_scratch(cfg, st, &mut scratch)
}

/// One full `Solute` call with caller-provided scratch buffers (zero heap allocations).
#[cfg(test)]
pub(crate) fn solute_step_with_scratch(
    cfg: &mut SoluteStepConfig,
    st: &mut SoluteStepState,
    scratch: &mut SoluteDriverScratch,
) -> Result<SoluteStepOutcome, KernelError> {
    validate(cfg, st)?;
    // Save backup copy in scratch so an error leaves the caller's state untouched.
    scratch.backup_state.copy_from(st);
    let mut c = cfg.clone();
    st.trace.clear();
    if let Err(e) = exclusion_call(&mut c, st) {
        st.copy_from(&scratch.backup_state);
        return Err(e);
    }
    st.peclet = 0.0;
    st.courant = 0.0;
    st.dt_max_c = 1.0e30;
    st.iter_c = 1;
    let outcome = match step_from_with_scratch(&c, st, 0, scratch) {
        Ok(out) => out,
        Err(e) => {
            st.copy_from(&scratch.backup_state);
            return Err(e);
        }
    };
    *cfg = c;
    Ok(outcome)
}

#[cfg(test)]
fn step_from_with_scratch(
    cfg: &SoluteStepConfig,
    working_st: &mut SoluteStepState,
    start: usize,
    scratch: &mut SoluteDriverScratch,
) -> Result<SoluteStepOutcome, KernelError> {
    for js in start..cfg.species_count {
        begin_species(cfg, working_st, js)?;
        let pass = solve_pass_with_scratch(cfg, working_st, js, 1, scratch)?;
        if !cfg.linear[js] && !pass.converged {
            return Ok(SoluteStepOutcome { pending: Some(js) });
        }
        finish_species_with_scratch(cfg, working_st, js, 1, scratch)?;
    }
    finish_step(cfg, working_st)?;
    Ok(SoluteStepOutcome { pending: None })
}

/// `HYDRUS.FOR:384-391`: `ThNew`/`vNew` hold the old level, then `Exclusion` runs under
/// `lBact.and..not.lWat` and rewrites all four node arrays and `ChPar(4,1)`.
#[cfg(test)]
#[allow(clippy::nonminimal_bool)]
fn exclusion_call(cfg: &mut SoluteStepConfig, st: &mut SoluteStepState) -> Result<(), KernelError> {
    if !(cfg.bact && !cfg.water_flow) {
        return Ok(());
    }
    let mut state = cfg
        .size_exclusion
        .clone()
        .ok_or("lBact without lWat needs the Exclusion primitive state")?;
    state.nodes = cfg.x.len();
    state.first_node_new_water = DefaultReal(st.th_o[0]);
    state.first_node_new_velocity = DefaultReal(st.v_o[0]);
    st.trace.push(DriverEvent::Exclusion);
    let result = size_exclusion(&state, cfg.bact, cfg.water_flow)?;
    // `1723` returned before any write: nothing changes, not even `ChPar(4,1)`.
    if result.as_ref().is_some_and(SizeExclusion::is_inactive) {
        return Ok(());
    }
    if let Some(SizeExclusion::Applied {
        new_water,
        old_water,
        new_velocity,
        old_velocity,
        colloid_exclusion_water_content,
        ..
    }) = result
    {
        let take = |v: Vec<DefaultReal>| v.into_iter().map(|d| d.0).collect::<Vec<f32>>();
        st.th_n = take(new_water);
        st.th_o = take(old_water);
        st.v_n = take(new_velocity);
        st.v_o = take(old_velocity);
        cfg.ch_par[0][3] = colloid_exclusion_water_content.0;
    }
    Ok(())
}

/// Species prologue (`SOLUTE.FOR:36-92`) and the `SetSSnk` call (94-96).
pub(crate) fn begin_species(
    cfg: &SoluteStepConfig,
    st: &mut SoluteStepState,
    js: usize,
) -> Result<(), KernelError> {
    let n = cfg.x.len();
    st.totals.cv_ch0[js] = DefaultReal(0.0);
    st.totals.cv_ch1[js] = DefaultReal(0.0);
    st.totals.cv_ch_r[js] = DefaultReal(0.0);
    st.totals.cv_ch_im[js] = DefaultReal(0.0);
    st.cv_top[js] = 0.0;
    st.cv_bot[js] = 0.0;
    // `t-tPulse.gt.dtMin` mixes a double `t` with default REALs.
    if st.t - cfg.t_pulse as f64 > cfg.dt_min as f64 && !cfg.atm_bc {
        st.c_top[js] = 0.0;
        st.c_bot[js] = 0.0;
    }
    if !cfg.linear[js] {
        for i in 0..n {
            st.c_new[i] = st.conc[js][i];
            if !cfg.equil {
                st.sorb_n[i] = st.sorb[js][i];
            }
            if cfg.bact || cfg.dual_neq {
                st.sorb_n2[i] = st.sorb2[js][i];
            }
        }
    }
    // `cvBot`/`cvTop` phase A is evaluated by the endpoint kernel from these snapshots.
    if let Some(ref mut frame) = st.frame {
        frame.v_o0.copy_from_slice(&st.v_o);
        frame.conc0.copy_from_slice(&st.conc[js]);
    } else {
        st.frame = Some(SpeciesFrame {
            v_o0: st.v_o.clone(),
            conc0: st.conc[js].clone(),
            endpoint: None,
        });
    }
    if cfg.root_sink {
        let mut state = cfg
            .root_uptake
            .clone()
            .ok_or("SinkF needs the SetSSnk primitive state")?;
        state.species = js + 1;
        state.species_count = cfg.species_count;
        state.time = DoubleReal(st.t);
        state.species_concentration = dr(&st.conc[js]);
        st.trace.push(DriverEvent::SetSSnk(js));
        if let Some(r) = root_solute_uptake(&state, true)? {
            st.s_sink = r.solute_sink.iter().map(|d| d.0).collect();
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LevelScalars {
    peclet: f32,
    courant: f32,
    dt_max: f32,
}

/// One `Coeff` call (`SOLUTE.FOR:241-750`), `Level` 1 (old) or 2 (new), writing directly into buffers.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::needless_range_loop
)]
fn coeff_level_into(
    cfg: &SoluteStepConfig,
    st: &mut SoluteStepState,
    js: usize,
    level: u8,
    iter: u32,
    disp: &mut [f32],
    retard: &mut [f32],
    g0: &mut [f32],
    g1: &mut [f32],
) -> Result<LevelScalars, KernelError> {
    let n = cfg.x.len();
    let fin = level == 2;
    let jjj = js * 16;
    let dt = st.dt;
    let epsi = cfg.epsi;
    let linear = cfg.linear[js];
    let mut peclet = 0.0_f32;
    let mut courant = 0.0_f32;
    let mut dt_max = 1.0e30_f32;
    let spec = BacteriaSpec {
        species: js,
        species_count: cfg.species_count,
        filtration: cfg.filtr,
        chpar: &cfg.ch_par,
        tdep: &cfg.t_dep,
    };
    for i in (0..n).rev() {
        let j = i + 1;
        let m = cfg.mat_num[i] - 1;
        let row = &cfg.ch_par[m];
        let temp_cur = if fin { st.temp_n[i] } else { st.temp_o[i] };
        let tt = tfac(temp_cur);
        let tto = tfac(st.temp_o[i]);
        let p = |slot: usize, t: f32| row[slot - 1] * (cfg.t_dep[slot - 1] * t).exp();
        let th_raw = if fin { st.th_n[i] } else { st.th_o[i] };
        let th_next_raw = (j < n).then(|| if fin { st.th_n[j] } else { st.th_o[j] });
        let (th_w, th_g, th_imob, th_imob_o, th_j) = if cfg.dual_por == 0 {
            let w = immobile_water(&ImmobileWaterSpec {
                final_level: fin,
                mobile_immobile: cfg.mobile_immobile[m],
                dual_porosity: 0,
                bacteria: cfg.bact,
                theta: th_raw,
                theta_sat: cfg.th_sat[m],
                chpar_th_imob: row[3],
                theta_next: th_next_raw,
            })?;
            (w.th_w, w.th_g, w.th_imob, w.th_imob_o, w.th_next)
        } else {
            let d = dual_domain_th_imob(fin, cfg.dual_por, cfg.bact, st.th_n_im[i], st.th_o_im[i])?
                .ok_or("iDualPor>0 immobile water selection failed")?;
            let nb = match th_next_raw {
                Some(t) => dual_neighbor_theta(cfg.dual_por, cfg.bact, t)?,
                None => None,
            };
            (
                th_raw,
                (cfg.th_sat[m] - th_raw).max(0.0),
                Some(d.th_imob),
                d.th_imob_o,
                nb,
            )
        };
        // `ThImob`/`ThImobO` are read only on branches that assigned them.
        let th_imob_f = th_imob.unwrap_or(0.0);
        let th_imob_o_f = th_imob_o.unwrap_or(0.0);
        let th_wo = st.th_o[i];
        let v = if fin { st.v_n[i] } else { st.v_o[i] };
        // `vj` is read after node `j` was corrected in this same call (descending loop).
        let vj = if j < n {
            if fin {
                st.v_n[j]
            } else {
                st.v_o[j]
            }
        } else {
            0.0
        };

        let cf = coeff_factors(
            cfg.moisture_selector,
            cfg.moisture.as_ref(),
            m,
            js,
            th_w,
            th_imob_f,
        )?;
        let ro = p(1, tt);
        let frac = p(3, tt);
        let dw = p(jjj + 5, tt);
        let mut dg = p(jjj + 6, tt);
        let xks = p(jjj + 7, tt);
        let xnu = p(jjj + 8, tt);
        let f_exp = row[jjj + 8];
        let henry = p(jjj + 10, tt);
        let gam_l = p(jjj + 11, tt) * cf.gam_l;
        let gam_li = p(jjj + 11, tt) * cf.gam_li;
        let gam_s = p(jjj + 12, tt) * cf.gam_s;
        let gam_si = p(jjj + 12, tt) * cf.gam_si;
        let mut gam_g = p(jjj + 13, tt) * cf.gam_g;
        let mut gam_l1 = p(jjj + 14, tt) * cf.gam_l1;
        let mut gam_l1i = p(jjj + 14, tt) * cf.gam_l1i;
        let mut gam_s1 = p(jjj + 15, tt) * cf.gam_s1;
        let mut gam_s1i = p(jjj + 15, tt) * cf.gam_s1i;
        let mut gam_g1 = p(jjj + 16, tt) * cf.gam_g1;
        let mut xmu_l = p(jjj + 17, tt) * cf.xmu_l;
        let mut xmu_s = p(jjj + 18, tt) * cf.xmu_s;
        let mut xmu_g = p(jjj + 19, tt) * cf.xmu_g;
        let ot = omega_terms(&OmegaSpec {
            final_level: fin,
            dual_neq: cfg.dual_neq,
            chpar_omega: row[jjj + 19],
            tdep_omega: cfg.t_dep[jjj + 19],
            chpar_omega_s: row[jjj + 15],
            tdep_omega_s: cfg.t_dep[jjj + 15],
            chpar_f_em: row[jjj + 12],
            tdep_f_em: cfg.t_dep[jjj + 12],
            tt,
            tto,
        })?;
        let mut omega = ot.omega;
        let omega_s = ot.omega_s.unwrap_or(0.0);
        let f_em = ot.f_em;

        // lBact new-level block (374-425).
        let mut bs = BacteriaScalars {
            s_max1: 0.0,
            s_max2: 0.0,
            r_ka1: 0.0,
            r_ka2: 0.0,
            r_kd1: 0.0,
            r_kd2: 0.0,
            psi1: 1.0,
            psi2: 1.0,
        };
        if cfg.bact {
            let node = BacteriaNode {
                material: m,
                temp_n: st.temp_n[i],
                temp_o: st.temp_o[i],
                x: cfg.x[i],
                sorb: st.sorb[js][i],
                sorb2: st.sorb2[js][i],
                sorb_n: st.sorb_n[i],
                sorb_n2: st.sorb_n2[i],
                final_level: fin,
                water: th_w,
                velocity: v,
            };
            let (lvl, z) = select_new_level(&spec, &node, st.carried)?;
            let b = |r: Option<super::biology::BlockingRequest>| {
                r.map(|b| blocking(b.i_psi, b.s_max, b.ss, b.x, b.dc, b.aa))
            };
            let dep = match lvl.deposit {
                Some(d) => Some(deposit(&DepositInput {
                    alfa1: d.alfa1,
                    alfa2: d.alfa2,
                    dc1: d.dc,
                    dp1: d.dp,
                    theta: d.theta,
                    q: d.q,
                    temp: d.temp,
                    x_conv: cfg.x_conv,
                    t_conv: cfg.t_conv,
                })?),
                None => None,
            };
            bs = lvl.finish(b(lvl.blocking1), b(lvl.blocking2), dep)?;
            st.carried = lvl.carried;
            dg = z.dg;
            gam_g = z.gam_g;
            gam_l1 = z.gam_l1;
            gam_l1i = z.gam_l1i;
            gam_s1 = z.gam_s1;
            gam_s1i = z.gam_s1i;
            gam_g1 = z.gam_g1;
            xmu_l = z.xmu_l;
            xmu_s = z.xmu_s;
            xmu_g = z.xmu_g;
            omega = z.omega;
        }

        // Previous species (434-456).
        let (mut xks_p, mut xnu_p, mut f_exp_p, mut henry_p) = (0.0, 0.0, 0.0, 0.0);
        let (mut gam_l1p, mut gam_l1pi, mut gam_s1p, mut gam_s1pi, mut gam_g1p) =
            (0.0, 0.0, 0.0, 0.0, 0.0);
        if js > 0 {
            let pj = (js - 1) * 16;
            let pf = prev_species_factors(
                cfg.moisture_selector,
                cfg.moisture.as_ref(),
                m,
                js,
                th_w,
                th_imob_f,
            )?;
            xks_p = p(pj + 7, tt);
            xnu_p = p(pj + 8, tt);
            f_exp_p = row[pj + 8];
            henry_p = p(pj + 10, tt);
            gam_l1p = p(pj + 14, tt) * pf.gam_l1p;
            gam_l1pi = p(pj + 14, tt) * pf.gam_l1pi;
            gam_s1p = p(pj + 15, tt) * pf.gam_s1p;
            gam_s1pi = p(pj + 15, tt) * pf.gam_s1pi;
            gam_g1p = p(pj + 16, tt) * pf.gam_g1p;
        }

        // Level-dependent old values and temperature derivatives (461-546).
        let (mut xks_o, mut xnu_o, mut f_exp_o) = (0.0_f32, 0.0_f32, 0.0_f32);
        let (mut gam_loi, mut gam_soi) = (0.0, 0.0);
        let (mut gam_l1oi, mut gam_s1oi) = (0.0, 0.0);
        let (mut xmu_lo, mut xmu_so, mut omega_o) = (0.0, 0.0, 0.0);
        let (mut rka1_o, mut rka2_o, mut rkd1_o, mut rkd2_o) = (0.0, 0.0, 0.0, 0.0);
        let (mut psi1_o, mut psi2_o) = (1.0_f32, 1.0_f32);
        let (d_ks, d_nu, dd_exp);
        let (tti, ttj);
        if fin {
            xks_o = p(jjj + 7, tto);
            xnu_o = p(jjj + 8, tto);
            f_exp_o = row[jjj + 8];
            let of = old_level_factors(
                cfg.moisture_selector,
                cfg.moisture.as_ref(),
                m,
                js,
                th_wo,
                th_imob_o_f,
            )?;
            gam_loi = p(jjj + 11, tto) * of.gam_loi;
            gam_soi = p(jjj + 12, tto) * of.gam_soi;
            gam_l1oi = p(jjj + 14, tto) * of.gam_l1oi;
            gam_s1oi = p(jjj + 15, tto) * of.gam_s1oi;
            xmu_lo = p(jjj + 17, tto) * of.xmu_lo;
            xmu_so = p(jjj + 18, tto) * of.xmu_so;
            omega_o = ot.omega_o.unwrap_or(0.0);
            d_ks = (xks - xks_o) / dt;
            d_nu = (xnu - xnu_o) / dt;
            dd_exp = (f_exp - f_exp_o) / dt;
            tti = (i > 0).then(|| tfac(st.temp_n[i - 1]));
            ttj = (j < n).then(|| tfac(st.temp_n[j]));
            if cfg.bact {
                let node = BacteriaNode {
                    material: m,
                    temp_n: st.temp_n[i],
                    temp_o: st.temp_o[i],
                    x: cfg.x[i],
                    sorb: st.sorb[js][i],
                    sorb2: st.sorb2[js][i],
                    sorb_n: st.sorb_n[i],
                    sorb_n2: st.sorb_n2[i],
                    final_level: true,
                    water: th_wo,
                    velocity: st.v_o[i],
                };
                let (lvl, z) = select_old_level(&spec, &node, st.carried)?;
                let b = |r: Option<super::biology::BlockingRequest>| {
                    r.map(|b| blocking(b.i_psi, b.s_max, b.ss, b.x, b.dc, b.aa))
                };
                let dep = match lvl.deposit {
                    Some(d) => Some(deposit(&DepositInput {
                        alfa1: d.alfa1,
                        alfa2: d.alfa2,
                        dc1: d.dc,
                        dp1: d.dp,
                        theta: d.theta,
                        q: d.q,
                        temp: d.temp,
                        x_conv: cfg.x_conv,
                        t_conv: cfg.t_conv,
                    })?),
                    None => None,
                };
                let o = lvl.finish(b(lvl.blocking1), b(lvl.blocking2), dep)?;
                st.carried = lvl.carried;
                rka1_o = o.r_ka1;
                rka2_o = o.r_ka2;
                rkd1_o = o.r_kd1;
                rkd2_o = o.r_kd2;
                psi1_o = o.psi1;
                psi2_o = o.psi2;
                gam_s1oi = z.gam_s1oi;
                xmu_lo = z.xmu_lo;
                xmu_so = z.xmu_so;
                omega_o = z.omega_o;
            }
        } else {
            let ttn = tfac(st.temp_n[i]);
            let xks_n = p(jjj + 7, ttn);
            let xnu_n = p(jjj + 8, ttn);
            let f_exp_n = row[jjj + 8];
            d_ks = (xks_n - xks) / dt;
            d_nu = (xnu_n - xnu) / dt;
            dd_exp = (f_exp_n - f_exp) / dt;
            tti = (i > 0).then(|| tfac(st.temp_o[i - 1]));
            ttj = (j < n).then(|| tfac(st.temp_o[j]));
        }
        let henry_at = |k: usize, t: f32| {
            let r = &cfg.ch_par[cfg.mat_num[k] - 1];
            r[jjj + 9] * (cfg.t_dep[jjj + 9] * t).exp()
        };
        let henry_i = tti.map(|t| henry_at(i - 1, t));
        let henry_j = ttj.map(|t| henry_at(j, t));

        // Nonlinear adsorption (561-622).
        if linear && st.c_mid.is_none() {
            return Err("cMid is read before any Coeff assignment (SOLUTE.FOR:573 vs 687)");
        }
        let mobile_or_dual = cfg.mobile_immobile[m] || cfg.dual_por > 0;
        let iso = isotherm_factors(&IsothermInput {
            linear,
            final_level: fin,
            equilibrium: cfg.equil,
            mobile_or_dual,
            conc: st.conc[js][i],
            c_new: st.c_new[i],
            sorb: st.sorb[js][i],
            sorb_n: st.sorb_n[i],
            f_exp,
            xnu,
            xks,
            f_exp_o,
            xnu_o,
            dks: d_ks,
            dnu: d_nu,
            dd_exp,
            c_mid_carried: st.c_mid.unwrap_or(0.0),
        });
        if !linear {
            st.c_mid = Some(iso.c_mid);
        }
        let c_prev = if js > 0 {
            if fin {
                st.conc[js - 1][i]
            } else {
                st.c_prev_o[i]
            }
        } else {
            0.0
        };
        let (sconc_p, sconc_ps) = if js > 0 {
            prev_species_isotherm(
                cfg.linear[js - 1],
                c_prev,
                st.sorb[js - 1][i],
                f_exp_p,
                xnu_p,
            )
        } else {
            (1.0, 1.0)
        };

        // Retard and Disper (625-629, 1025-1067).
        let node = TransportNode {
            water: th_w,
            gas_water: th_g,
            saturation_water: cfg.th_sat[m],
            immobile_water: th_imob_f,
            velocity: v,
            dispersivity: row[1],
            molecular_diffusion: dw,
            gas_diffusion: dg,
            henry,
            bulk_density: ro,
            equilibrium_fraction: frac,
            f_em,
            sorption_coefficient: xks,
            exponent: f_exp,
            sorption_nu: xnu,
            concentration_midpoint: iso.c_mid,
            concentration_derivative: iso.dconc,
            sorption_derivative: iso.dsconc,
            process_correction: 0.0,
            tortuosity: cfg.tort,
            tortuosity_method: cfg.i_tort,
            mobile_immobile: cfg.mobile_immobile[m],
            bacteria: cfg.bact,
            dual_domain: cfg.dual_por > 0,
        };
        let tc = coefficients(node, dt, fin, cfg.art_disp, cfg.upwind, cfg.pe_cr);
        retard[i] = tc.retardation;
        disp[i] = tc.dispersion;

        // NEquil (631-641).
        let (mut ssorb, mut ssorb2, mut fl_macro) = (0.0_f32, 0.0_f32, 0.0_f32);
        if should_call_nequil(cfg.equil) {
            let flags = NEquilFlags {
                linear,
                mobile_immobile: cfg.mobile_immobile[m],
                dual_porosity: cfg.dual_por,
                bacteria: cfg.bact,
                dual_neq: cfg.dual_neq,
                chain: false,
                final_level: fin,
            };
            let input = NEquilInput {
                dt,
                epsi,
                ro,
                frac,
                xks,
                xks_o,
                xks_p,
                cc: iso.cc,
                sconc: iso.sconc,
                sconc_o: iso.sconc_o,
                sconc_s: iso.sconc_s,
                sconc_o_s: iso.sconc_o_s,
                sconc_ps,
                dsconc_s: iso.dsconc_s,
                xmu_l,
                xmu_l_o: xmu_lo,
                xmu_s,
                xmu_s_o: xmu_so,
                dretard_s: iso.dretard_s,
                gam_l: gam_li,
                gam_l1: gam_l1i,
                gam_l_o: gam_loi,
                gam_l1_o: gam_l1oi,
                gam_s: gam_si,
                gam_s1: gam_s1i,
                gam_s_o: gam_soi,
                gam_s1_o: gam_s1oi,
                gam_l1_pi: gam_l1pi,
                gam_s1_pi: gam_s1pi,
                omega,
                omega_o,
                omega_s,
                f_em,
                rka1: bs.r_ka1,
                rka1_o,
                rka2: bs.r_ka2,
                rka2_o,
                rkd1: bs.r_kd1,
                rkd1_o,
                rkd2: bs.r_kd2,
                rkd2_o,
                th_w,
                th_w_o: th_wo,
                psi1: bs.psi1,
                psi1_o,
                psi2: bs.psi2,
                psi2_o,
                th_imob: th_imob_f,
                th_imob_o: th_imob_o_f,
                dmob_i: st.dmob_i,
            };
            let mut pools = NEquilPools {
                conc: &st.conc,
                sorb: &mut st.sorb,
                sorb2: &mut st.sorb2,
                c_new: &st.c_new,
                sorb_n: &mut st.sorb_n,
                sorb_n2: &mut st.sorb_n2,
                sink_im: &st.sink_im,
            };
            let o = nequil(js, i, &flags, &input, &mut pools)?;
            ssorb = o.ssorb;
            ssorb2 = o.ssorb2;
            fl_macro = o.fl_macro;
            st.dmob_i = o.dmob_i;
        }

        // g0, q0, g1, q1 (643-730).
        let chain = if js > 0 {
            chain_source(&ChainSourceInput {
                species: js,
                equilibrium: cfg.equil,
                mobile_or_dual,
                bacteria: cfg.bact,
                chain_noneq: false,
                dual_neq: cfg.dual_neq,
                c_prev,
                gam_l1p,
                gam_l1pi,
                gam_s1p,
                gam_s1pi,
                gam_g1p,
                henry_p,
                xks_p,
                sconc_p,
                sconc_ps,
                th_w,
                th_g,
                th_imob: th_imob_f,
                ro,
                frac,
                f_em,
                sorb_prev: st.sorb[js - 1][i],
                sorb2_prev: st.sorb2[js - 1][i],
            })?
        } else {
            None
        };
        let terms = source_terms(&SourceTermsInput {
            equilibrium: cfg.equil,
            linear,
            final_level: fin,
            mobile_or_dual,
            dual_porosity: cfg.dual_por,
            bacteria: cfg.bact,
            dual_neq: cfg.dual_neq,
            dt,
            conc: st.conc[js][i],
            sorb: st.sorb[js][i],
            sorb2: st.sorb2[js][i],
            s_sink: st.s_sink[i],
            sink_im: st.sink_im[i],
            th_w,
            th_g,
            th_imob: th_imob_f,
            ro,
            frac,
            f_em,
            xks,
            henry,
            xmu_l,
            xmu_s,
            xmu_g,
            gam_l,
            gam_l1,
            gam_s,
            gam_s1,
            gam_g,
            gam_g1,
            gam_li,
            gam_l1i,
            gam_si,
            gam_s1i,
            omega,
            omega_s,
            rka1: bs.r_ka1,
            rka2: bs.r_ka2,
            rkd1: bs.r_kd1,
            rkd2: bs.r_kd2,
            psi1: bs.psi1,
            psi2: bs.psi2,
            ssorb,
            ssorb2,
            fl_macro,
            dmob_i: st.dmob_i,
            sconc: iso.sconc,
            sconc_s: iso.sconc_s,
            dretard: iso.dretard,
            c_mid: iso.c_mid,
            chain,
        });
        g0[i] = terms.g0;
        g1[i] = terms.g1;
        st.q0[i] = terms.q0;
        st.q1[i] = terms.q1;

        // Velocity corrections (732-745).
        let (dx, der_k) = if i == 0 {
            let dx = cfg.x[1] - cfg.x[0];
            (dx, (henry_j.unwrap_or(0.0) - henry) / dx)
        } else if i == n - 1 {
            let dx = cfg.x[n - 1] - cfg.x[n - 2];
            (dx, (henry - henry_i.unwrap_or(0.0)) / dx)
        } else {
            let dx = (cfg.x[j] - cfg.x[i - 1]) / 2.0;
            (dx, (henry_j.unwrap_or(0.0) - henry_i.unwrap_or(0.0)) / dx)
        };
        let _ = dx;
        let vc = th_g * dg * tc.tau_gas * der_k;
        st.v_corr[i] = vc;
        if level == 1 {
            st.v_o[i] -= vc;
        } else {
            st.v_n[i] -= vc;
        }

        // PeCour (748-749, 965-1016).
        if j < n {
            let pc = pecour(PecletInputs {
                dx: cfg.x[j] - cfg.x[i],
                dt,
                velocity: v,
                neighbor_velocity: vj,
                water: th_w,
                neighbor_water: th_j.unwrap_or(0.0),
                dispersion: disp[i],
                neighbor_dispersion: disp[j],
                retardation: retard[i],
                neighbor_retardation: retard[j],
                upstream_weighting: cfg.upwind,
                artificial_dispersion: cfg.art_disp,
                level_is_new: fin,
                iteration: iter as usize,
                epsi,
                threshold: cfg.pe_cr,
            });
            if fin {
                peclet = peclet.max(pc.peclet_max);
                courant = courant.max(pc.courant_max);
                dt_max = dt_max.min(pc.dt_max);
            } else if let Some(w) = pc.weight {
                st.wc[i] = w;
            }
        }
    }
    Ok(LevelScalars {
        peclet,
        courant,
        dt_max,
    })
}

/// `MatSet` water adjustment (`768-776`), applied to the interior rows only; the
/// endpoint kernel applies it itself.
fn adjusted_into(cfg: &SoluteStepConfig, th: &[f32], dst: &mut [f32]) {
    for (i, d) in dst.iter_mut().enumerate().take(th.len()) {
        let m = cfg.mat_num[i] - 1;
        *d = if cfg.mobile_immobile[m] && cfg.dual_por == 0 || cfg.bact {
            (th[i] - cfg.ch_par[m][3]).max(0.001)
        } else {
            th[i]
        };
    }
}

fn mass_tran_input<'a>(
    cfg: &'a SoluteStepConfig,
    js: usize,
    bufs: &'a MassBufs,
    old: bool,
) -> MassTranInput<'a> {
    MassTranInput {
        species: js,
        coordinates: &bufs.x,
        material_ids: &bufs.mat0,
        temperature: if old { &bufs.temp_o } else { &bufs.temp_n },
        chpar: &bufs.chpar,
        tdep: &bufs.tdep,
        mobile_immobile_by_material: &cfg.mobile_immobile,
        equilibrium: cfg.equil,
        concentration: &bufs.conc,
        sorb: &bufs.sorb,
        sorb2: &bufs.sorb2,
        q0: &bufs.q0,
        q1: &bufs.q1,
        sink: &bufs.s_sink,
        theta: if old { &bufs.th_o } else { &bufs.th_n },
        velocity: if old { &bufs.v_o } else { &bufs.v_n },
        filtration: cfg.filtr,
        bacteria: cfg.bact,
        dual_neq: cfg.dual_neq,
        dual_porosity: (cfg.dual_por > 0).then_some(cfg.dual_por),
        sink_immobile: &bufs.sink_im,
        linear: cfg.linear[js],
        x_conv: DefaultReal(cfg.x_conv),
        t_conv: DefaultReal(cfg.t_conv),
        epsi: DefaultReal(if old { 1.0 - cfg.epsi } else { cfg.epsi }),
    }
}

/// Owned `DefaultReal` views of the arrays a `MassTran` call reads.
pub(crate) struct MassBufs {
    pub(crate) x: Vec<DefaultReal>,
    pub(crate) mat0: Vec<usize>,
    pub(crate) temp_o: Vec<DefaultReal>,
    pub(crate) temp_n: Vec<DefaultReal>,
    pub(crate) chpar: Vec<Vec<DefaultReal>>,
    pub(crate) tdep: Vec<DefaultReal>,
    pub(crate) conc: Vec<DefaultReal>,
    pub(crate) sorb: Vec<DefaultReal>,
    pub(crate) sorb2: Vec<DefaultReal>,
    pub(crate) q0: Vec<DefaultReal>,
    pub(crate) q1: Vec<DefaultReal>,
    pub(crate) s_sink: Vec<DefaultReal>,
    pub(crate) th_o: Vec<DefaultReal>,
    pub(crate) th_n: Vec<DefaultReal>,
    pub(crate) v_o: Vec<DefaultReal>,
    pub(crate) v_n: Vec<DefaultReal>,
    pub(crate) sink_im: Vec<DefaultReal>,
}

fn copy_slice_dr(dst: &mut [DefaultReal], src: &[f32]) {
    for (d, &s) in dst.iter_mut().zip(src) {
        *d = DefaultReal(s);
    }
}

fn mass_bufs_into(cfg: &SoluteStepConfig, st: &SoluteStepState, js: usize, bufs: &mut MassBufs) {
    copy_slice_dr(&mut bufs.x, &cfg.x);
    for (d, &m) in bufs.mat0.iter_mut().zip(&cfg.mat_num) {
        *d = m - 1;
    }
    copy_slice_dr(&mut bufs.temp_o, &st.temp_o);
    copy_slice_dr(&mut bufs.temp_n, &st.temp_n);
    for (d_row, s_row) in bufs.chpar.iter_mut().zip(&cfg.ch_par) {
        copy_slice_dr(d_row, s_row);
    }
    copy_slice_dr(&mut bufs.tdep, &cfg.t_dep);
    copy_slice_dr(&mut bufs.conc, &st.conc[js]);
    copy_slice_dr(&mut bufs.sorb, &st.sorb[js]);
    copy_slice_dr(&mut bufs.sorb2, &st.sorb2[js]);
    copy_slice_dr(&mut bufs.q0, &st.q0);
    copy_slice_dr(&mut bufs.q1, &st.q1);
    copy_slice_dr(&mut bufs.s_sink, &st.s_sink);
    copy_slice_dr(&mut bufs.th_o, &st.th_o);
    copy_slice_dr(&mut bufs.th_n, &st.th_n);
    copy_slice_dr(&mut bufs.v_o, &st.v_o);
    copy_slice_dr(&mut bufs.v_n, &st.v_n);
    copy_slice_dr(&mut bufs.sink_im, &st.sink_im);
}

fn sync_s_trans_in(st: &mut SoluteStepState) {
    if st.totals.s_trans.len() == st.s_trans.len() {
        for (d, &s) in st.totals.s_trans.iter_mut().zip(&st.s_trans) {
            *d = DefaultReal(s);
        }
    } else {
        st.totals.s_trans = dr(&st.s_trans);
    }
}

fn sync_s_trans_out(st: &mut SoluteStepState) {
    if st.s_trans.len() == st.totals.s_trans.len() {
        for (s, d) in st.s_trans.iter_mut().zip(&st.totals.s_trans) {
            *s = d.0;
        }
    } else {
        st.s_trans = st.totals.s_trans.iter().map(|d| d.0).collect();
    }
}

/// One execution of labels 12-16 for species `js`.
#[cfg(test)]
pub(crate) fn solve_pass(
    cfg: &SoluteStepConfig,
    st: &mut SoluteStepState,
    js: usize,
    iter: u32,
) -> Result<PassOutcome, KernelError> {
    let mut scratch = SoluteDriverScratch::new(cfg.x.len(), cfg.species_count, cfg.th_sat.len());
    solve_pass_with_scratch(cfg, st, js, iter, &mut scratch)
}

/// One execution of labels 12-16 for species `js` with preallocated scratch (zero heap allocations).
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub(crate) fn solve_pass_with_scratch(
    cfg: &SoluteStepConfig,
    st: &mut SoluteStepState,
    js: usize,
    iter: u32,
    scratch: &mut SoluteDriverScratch,
) -> Result<PassOutcome, KernelError> {
    validate(cfg, st)?;
    if st.frame.is_none() {
        return Err("solve_pass needs begin_species to have run");
    }
    let n = cfg.x.len();
    let alf = 1.0 - cfg.epsi;
    let linear = cfg.linear[js];
    if !linear {
        st.c_temp.copy_from_slice(&st.c_new);
    }

    // ---- Level 1 ----
    let _l1 = coeff_level_into(
        cfg,
        st,
        js,
        1,
        iter,
        &mut scratch.l1_disp,
        &mut scratch.l1_retard,
        &mut scratch.l1_g0,
        &mut scratch.l1_g1,
    )?;
    st.trace.push(DriverEvent::Coeff(js, 1));
    adjusted_into(cfg, &st.th_o, &mut scratch.th_o_adj);
    adjusted_into(cfg, &st.th_n, &mut scratch.th_n_adj);
    scratch.old_matrix_v.copy_from_slice(&st.v_o);
    scratch.new_matrix_v.copy_from_slice(&st.v_n);

    scratch.rows_old_rhs.fill(0.0);
    assemble_interior_into(
        AssemblyInputs {
            x: &cfg.x,
            theta_old: &scratch.th_o_adj,
            theta_new: &scratch.th_n_adj,
            velocity_old: &scratch.old_matrix_v,
            velocity_new: &scratch.new_matrix_v,
            concentration: &st.conc[js],
            dispersion: &scratch.l1_disp,
            retardation: &scratch.l1_retard,
            upstream_weight: &st.wc,
            g0: &scratch.l1_g0,
            g1: &scratch.l1_g1,
        },
        AssemblyLevel::Old,
        cfg.epsi,
        alf,
        st.dt,
        &mut scratch.lower,
        &mut scratch.diag,
        &mut scratch.upper,
        &mut scratch.rows_old_rhs,
    )?;
    st.trace.push(DriverEvent::MatSet(js, 1));
    for i in 0..n {
        st.v_o[i] += st.v_corr[i];
    }
    // Beginning MassTran (135-140): old temperature, water, velocity and pools, weight alf.
    {
        sync_s_trans_in(st);
        mass_bufs_into(cfg, st, js, &mut scratch.mass_bufs);
        let input = mass_tran_input(cfg, js, &scratch.mass_bufs, true);
        let ran = st
            .totals
            .accumulate_step_start(1, iter, &input, DefaultReal(alf))?;
        sync_s_trans_out(st);
        if ran {
            st.trace.push(DriverEvent::MassTranBegin(js));
        }
    }

    // ---- Level 2 ----
    let l2 = coeff_level_into(
        cfg,
        st,
        js,
        2,
        iter,
        &mut scratch.l2_disp,
        &mut scratch.l2_retard,
        &mut scratch.l2_g0,
        &mut scratch.l2_g1,
    )?;
    st.trace.push(DriverEvent::Coeff(js, 2));
    st.peclet = st.peclet.max(l2.peclet);
    st.courant = st.courant.max(l2.courant);
    st.dt_max_c = st.dt_max_c.min(l2.dt_max);
    scratch.new_matrix_v.copy_from_slice(&st.v_n);

    scratch.rhs.copy_from_slice(&scratch.rows_old_rhs);
    assemble_interior_into(
        AssemblyInputs {
            x: &cfg.x,
            theta_old: &scratch.th_o_adj,
            theta_new: &scratch.th_n_adj,
            velocity_old: &st.v_o,
            velocity_new: &scratch.new_matrix_v,
            concentration: &st.conc[js],
            dispersion: &scratch.l2_disp,
            retardation: &scratch.l2_retard,
            upstream_weight: &st.wc,
            g0: &scratch.l2_g0,
            g1: &scratch.l2_g1,
        },
        AssemblyLevel::New,
        cfg.epsi,
        alf,
        st.dt,
        &mut scratch.lower,
        &mut scratch.diag,
        &mut scratch.upper,
        &mut scratch.rhs,
    )?;
    for i in 0..n {
        st.v_n[i] += st.v_corr[i];
    }

    let jjj = js * 16;
    let top_atm = cfg.k_top == -2;
    let frame = st.frame.as_ref().expect("checked above");
    let ep = &mut scratch.endpoint;
    copy_slice_dr(&mut ep.coordinates, &cfg.x);
    copy_slice_dr(&mut ep.old_water, &st.th_o);
    copy_slice_dr(&mut ep.new_water, &st.th_n);
    copy_slice_dr(&mut ep.old_velocity, &frame.v_o0);
    copy_slice_dr(&mut ep.new_velocity, &st.v_n);
    copy_slice_dr(&mut ep.old_matrix_velocity, &scratch.old_matrix_v);
    copy_slice_dr(&mut ep.new_matrix_velocity, &scratch.new_matrix_v);
    copy_slice_dr(&mut ep.old_dispersion, &scratch.l1_disp);
    copy_slice_dr(&mut ep.new_dispersion, &scratch.l2_disp);
    copy_slice_dr(&mut ep.old_retardation, &scratch.l1_retard);
    copy_slice_dr(&mut ep.new_retardation, &scratch.l2_retard);
    copy_slice_dr(&mut ep.old_concentration, &frame.conc0);
    copy_slice_dr(&mut ep.new_accepted_concentration, &frame.conc0);
    for (d, r) in ep.material_immobile_water.iter_mut().zip(&cfg.ch_par) {
        *d = DefaultReal(r[3]);
    }
    copy_slice_dr(&mut ep.old_source_g0, &scratch.l1_g0);
    copy_slice_dr(&mut ep.old_source_g1, &scratch.l1_g1);
    copy_slice_dr(&mut ep.new_source_g0, &scratch.l2_g0);
    copy_slice_dr(&mut ep.new_source_g1, &scratch.l2_g1);
    copy_slice_dr(&mut ep.upwind_weight, &st.wc[..n - 1]);
    ep.epsi = DefaultReal(cfg.epsi);
    ep.dt = DefaultReal(st.dt);
    ep.alf = DefaultReal(alf);
    ep.top_atmospheric_concentration = top_atm.then_some(DefaultReal(cfg.c_atm));
    ep.top_surface_distance = top_atm.then_some(DefaultReal(cfg.d_surf));
    ep.old_temperature = top_atm.then_some(DefaultReal(st.temp_o[n - 1]));
    ep.new_temperature = top_atm.then_some(DefaultReal(st.temp_n[n - 1]));
    ep.source_chpar = top_atm.then(|| {
        cfg.ch_par
            .iter()
            .enumerate()
            .map(|(m, r)| MaterialSpeciesParameters {
                species: js + 1,
                material: m + 1,
                slots: dr(&r[jjj..jjj + 20]),
            })
            .collect()
    });
    ep.source_tdep = top_atm.then(|| dr(&cfg.t_dep));
    ep.material_ids.copy_from_slice(&cfg.mat_num);
    ep.source_species = js + 1;
    ep.mobile_immobile.copy_from_slice(&cfg.mobile_immobile);
    ep.bacteria = cfg.bact;
    ep.dual_porosity = cfg.dual_por;
    ep.bottom_resistance = DefaultReal(cfg.r_bot);
    ep.bottom_prescribed = DefaultReal(st.c_bot[js]);
    ep.top_prescribed = DefaultReal(st.c_top[js]);
    ep.t_level = cfg.t_level;
    ep.vapor = cfg.vapor;
    ep.r_min = DefaultReal(R_MIN);

    let ends = assemble_endpoints(ep, cfg.k_bot, cfg.k_top)?;
    st.trace.push(DriverEvent::MatSet(js, 2));

    for (idx, row) in [(0, ends.lower.assembled), (n - 1, ends.upper.assembled)] {
        scratch.lower[idx] = row.lower;
        scratch.diag[idx] = row.diagonal;
        scratch.upper[idx] = row.upper;
        scratch.rhs[idx] = row.rhs;
    }
    ban_solve(
        &mut scratch.lower,
        &mut scratch.diag,
        &mut scratch.upper,
        &mut scratch.rhs,
    )?;
    st.trace.push(DriverEvent::BanSol(js));
    // Pre-allocated endpoint in frame avoids cloning ep on subsequent iterations
    if let Some(frame) = st.frame.as_mut() {
        if let Some(ref mut dst) = frame.endpoint {
            copy_endpoint_in_place(dst, ep);
        } else {
            // First time this frame sees an endpoint, clone into it
            frame.endpoint = Some(ep.clone());
        }
    }

    // Convergence test and Conc update (151-179).
    let mut converged = true;
    for i in 0..n {
        if (cfg.species_count > 1 && iter == 1) || cfg.density {
            st.c_prev_o[i] = st.conc[js][i];
        }
        let f = scratch.rhs[i] as f32;
        if linear {
            let mut c = if f > 0.0 { f } else { 0.0 };
            if c < 1.0e-30 && c > 0.0 {
                c = 0.0;
            }
            st.conc[js][i] = c;
        } else {
            let mut c = f;
            if c < 1.0e-30 {
                c = 0.0;
            }
            st.c_new[i] = c;
            if (c - st.c_temp[i]).abs() > cfg.c_tol_a + cfg.c_tol_r * st.conc[js][i] {
                converged = false;
            }
        }
    }
    if linear {
        st.trace.push(DriverEvent::Update(js));
    }
    Ok(PassOutcome { converged })
}

/// Label 17 and everything after it for one accepted species (`SOLUTE.FOR:184-225`).
#[cfg(test)]
pub(crate) fn finish_species(
    cfg: &SoluteStepConfig,
    st: &mut SoluteStepState,
    js: usize,
    iter: u32,
) -> Result<(), KernelError> {
    let mut scratch = SoluteDriverScratch::new(cfg.x.len(), cfg.species_count, cfg.th_sat.len());
    finish_species_with_scratch(cfg, st, js, iter, &mut scratch)
}

/// Zero-allocation implementation of `finish_species` using preallocated scratch buffers.
pub(crate) fn finish_species_with_scratch(
    cfg: &SoluteStepConfig,
    st: &mut SoluteStepState,
    js: usize,
    iter: u32,
    scratch: &mut SoluteDriverScratch,
) -> Result<(), KernelError> {
    validate(cfg, st)?;
    let n = cfg.x.len();
    if !cfg.linear[js] {
        for i in 0..n {
            st.conc[js][i] = st.c_new[i];
            if !cfg.equil {
                st.sorb[js][i] = st.sorb_n[i];
            }
            if cfg.bact || cfg.dual_neq {
                st.sorb2[js][i] = st.sorb_n2[i];
            }
        }
        st.trace.push(DriverEvent::Update(js));
    }
    if should_call_sorb_conc(cfg.equil, cfg.linear[js]) {
        let jjj = js * 16;
        let deposited = if cfg.bact && cfg.filtr {
            scratch.deposited.clear();
            for i in 0..n {
                let m = cfg.mat_num[i] - 1;
                let row = &cfg.ch_par[m];
                let tt = tfac(st.temp_n[i]);
                let p = |slot: usize| row[slot - 1] * (cfg.t_dep[slot - 1] * tt).exp();
                scratch.deposited.push(deposit(&DepositInput {
                    alfa1: p(jjj + 19),
                    alfa2: p(jjj + 16),
                    dc1: p(jjj + 13),
                    dp1: p(jjj + 14),
                    theta: st.th_n[i] - row[3],
                    q: st.v_n[i],
                    temp: st.temp_n[i],
                    x_conv: cfg.x_conv,
                    t_conv: cfg.t_conv,
                })?);
            }
            Some(scratch.deposited.as_slice())
        } else {
            None
        };
        for (d, &m) in scratch.mass_bufs.mat0.iter_mut().zip(&cfg.mat_num) {
            *d = m - 1;
        }
        let spec = SorbConcSpec {
            species: js,
            dt: st.dt,
            mobile_immobile: &cfg.mobile_immobile,
            bacteria: cfg.bact,
            filtration: cfg.filtr,
            dual_porosity: cfg.dual_por,
            moisture_selector: cfg.moisture_selector,
            dual_neq: cfg.dual_neq,
            chpar: &cfg.ch_par,
            tdep: &cfg.t_dep,
            moisture: cfg.moisture.as_ref(),
        };
        scratch.sorb2_scratch.copy_from_slice(&st.sorb2[js]);
        let mut sorb = std::mem::take(&mut st.sorb[js]);
        {
            let mut state = SorbConcState {
                material: &scratch.mass_bufs.mat0[..n],
                temperature: &st.temp_n,
                water: &st.th_n,
                immobile_new: &st.th_n_im,
                immobile_old: &st.th_o_im,
                sink_im: &st.sink_im,
                conc: &st.conc[js],
                sorb: &mut sorb,
                sorb2: &mut scratch.sorb2_scratch,
                s_trans: &mut st.s_trans,
                deposited_ka: deposited,
            };
            sorb_conc(&spec, &mut state)?;
        }
        st.sorb[js] = sorb;
        if cfg.bact || cfg.dual_neq {
            st.sorb2[js].copy_from_slice(&scratch.sorb2_scratch);
        }
        st.trace.push(DriverEvent::SorbConc(js));
    }

    // End-of-step MassTran (189-193).
    {
        sync_s_trans_in(st);
        mass_bufs_into(cfg, st, js, &mut scratch.mass_bufs);
        let input = mass_tran_input(cfg, js, &scratch.mass_bufs, false);
        st.totals.accumulate(&input)?;
        sync_s_trans_out(st);
        st.trace.push(DriverEvent::MassTranEnd(js));
    }

    // Flux closure (195-225): evaluate endpoints from the completed pass.
    let ends = if let Some(ref mut ep) = st.frame.as_mut().and_then(|f| f.endpoint.as_mut()) {
        copy_slice_dr(&mut ep.new_accepted_concentration, &st.conc[js]);
        assemble_endpoints(ep, cfg.k_bot, cfg.k_top)?
    } else {
        copy_slice_dr(
            &mut scratch.endpoint.new_accepted_concentration,
            &st.conc[js],
        );
        assemble_endpoints(&scratch.endpoint, cfg.k_bot, cfg.k_top)?
    };
    st.cv_top[js] = ends.flux.top as f32;
    st.cv_bot[js] = ends.flux.bottom as f32;
    if st.cv_top[js].abs() < R_MIN {
        st.cv_top[js] = 0.0;
    }
    if st.cv_bot[js].abs() < R_MIN {
        st.cv_bot[js] = 0.0;
    }
    st.iter_c = st.iter_c.max(iter);
    st.trace.push(DriverEvent::FluxClose(js));
    Ok(())
}

/// `if(iConcType.eq.2) call FluxConc(..., ThNIm, 1)` after the species loop (236-238).
pub(crate) fn finish_step(
    cfg: &SoluteStepConfig,
    st: &mut SoluteStepState,
) -> Result<(), KernelError> {
    if cfg.conc_type != 2 {
        return Ok(());
    }
    let x = dr(&cfg.x);
    let conc = dr(&st.conc[0]);
    let v = dr(&st.v_n);
    let water = dr(&st.th_n);
    let sat = dr(&cfg.th_sat);
    let chpar: Vec<Vec<DefaultReal>> = cfg.ch_par.iter().map(|r| dr(r)).collect();
    let tdep = dr(&cfg.t_dep);
    let mat0: Vec<usize> = cfg.mat_num.iter().map(|m| m - 1).collect();
    let temp = dr(&st.temp_n);
    let im = dr(&st.th_n_im);
    let imob_by_mat: Vec<DefaultReal> = cfg.ch_par.iter().map(|r| DefaultReal(r[3])).collect();
    let input = FluxConcInput {
        concentration: &conc,
        coordinates: &x,
        velocity: &v,
        water: &water,
        saturated_water_by_material: &sat,
        chpar_by_material: &chpar,
        tdep: &tdep,
        material_ids: &mat0,
        temperature: &temp,
        tortuosity: cfg.tort,
        mobile_immobile: &cfg.mobile_immobile,
        dual_domain_model: (cfg.dual_por > 0).then_some(5 + cfg.dual_por),
        dual_immobile_water: (cfg.dual_por > 0).then_some(im.as_slice()),
        immobile_water_by_material: &imob_by_mat,
    };
    if let Explicit::Available(c) = flux_concentration(&input)? {
        st.c_new = c.iter().map(|d| d.0).collect();
    }
    st.trace.push(DriverEvent::FluxConc);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use DriverEvent::*;

    const N: usize = 4;

    /// Slot values (1-based) of the single material: `ro=1.5`, `Disp=0.1`, `Frac=1`,
    /// `Dw=0.01`, `Dg=0.5`, `xKs=0.3`, `fExp=1`, `xMuL=0.01`, `Omega=0.2`.
    fn row(henry: f32) -> Vec<f32> {
        let mut r = vec![0.0_f32; 36];
        for (k, v) in [
            (1, 1.5),
            (2, 0.1),
            (3, 1.0),
            (5, 0.01),
            (6, 0.5),
            (7, 0.3),
            (9, 1.0),
            (10, henry),
            (17, 0.01),
            (20, 0.2),
        ] {
            r[k - 1] = v;
        }
        r
    }

    fn cfg() -> SoluteStepConfig {
        SoluteStepConfig {
            x: vec![0.0, 1.0, 2.0, 3.0],
            mat_num: vec![1; N],
            th_sat: vec![0.5],
            mobile_immobile: vec![false],
            ch_par: vec![row(0.0)],
            t_dep: vec![0.0; 36],
            species_count: 1,
            linear: vec![true],
            equil: true,
            upwind: false,
            art_disp: false,
            tort: false,
            i_tort: 0,
            bact: false,
            filtr: false,
            dual_neq: false,
            water_flow: true,
            root_sink: false,
            vapor: false,
            atm_bc: false,
            density: false,
            dual_por: 0,
            conc_type: 0,
            moisture_selector: 0,
            moisture: None,
            epsi: 0.5,
            t_pulse: 100.0,
            dt_min: 0.001,
            k_bot: 1,
            k_top: -1,
            c_atm: 0.0,
            d_surf: 1.0,
            r_bot: 0.0,
            t_level: 2,
            pe_cr: 2.0,
            x_conv: 1.0,
            t_conv: 1.0,
            c_tol_a: 0.0,
            c_tol_r: 0.0,
            root_uptake: None,
            size_exclusion: None,
        }
    }

    fn state() -> SoluteStepState {
        let mut s = SoluteStepState::blank(N, 1);
        s.conc[0] = vec![1.0, 0.8, 0.5, 0.2];
        s.th_o = vec![0.3; N];
        s.th_n = vec![0.32; N];
        s.v_o = vec![0.1; N];
        s.v_n = vec![0.1; N];
        s.temp_o = vec![20.0; N];
        s.temp_n = vec![20.0; N];
        s.c_bot = vec![1.0];
        s.t = 1.0;
        s.dt = 0.1;
        s.c_mid = Some(0.0);
        s
    }

    fn run(c: &mut SoluteStepConfig, s: &mut SoluteStepState) -> SoluteStepOutcome {
        solute_step(c, s).expect("step")
    }

    #[test]
    fn linear_equilibrium_follows_the_source_call_order() {
        let (mut c, mut s) = (cfg(), state());
        assert_eq!(run(&mut c, &mut s).pending, None);
        assert_eq!(
            s.trace,
            vec![
                Coeff(0, 1),
                MatSet(0, 1),
                MassTranBegin(0),
                Coeff(0, 2),
                MatSet(0, 2),
                BanSol(0),
                Update(0),
                MassTranEnd(0),
                FluxClose(0)
            ]
        );
        assert!(s.conc[0].iter().all(|c| c.is_finite() && *c >= 0.0));
    }

    #[test]
    fn beginning_mass_tran_adds_alf_weighted_totals_once() {
        // q0 = xMuL*ThW = 0.01*0.3 (Level 1) and 0.01*0.32 (Level 2); three unit
        // elements; weights alf = epsi = 0.5.
        let (mut c, mut s) = (cfg(), state());
        run(&mut c, &mut s);
        let begin = 0.5_f32 * 3.0 * 0.003;
        let end = 0.5_f32 * 3.0 * 0.0032;
        let got = s.totals.cv_ch0[0].0;
        assert!((got - (begin + end)).abs() < 1e-6, "{got}");
    }

    #[test]
    fn beginning_mass_tran_is_skipped_after_the_first_iteration() {
        let (c, mut s) = (cfg(), state());
        begin_species(&c, &mut s, 0).unwrap();
        solve_pass(&c, &mut s, 0, 2).unwrap();
        assert!(!s.trace.contains(&MassTranBegin(0)));
        assert_eq!(s.totals.cv_ch0[0].0, 0.0);
        let mut s1 = state();
        begin_species(&c, &mut s1, 0).unwrap();
        solve_pass(&c, &mut s1, 0, 1).unwrap();
        assert_eq!(
            s1.trace.iter().filter(|e| **e == MassTranBegin(0)).count(),
            1
        );
    }

    #[test]
    fn two_site_sorb_conc_sits_between_the_update_and_the_end_mass_tran() {
        let (mut c, mut s) = (cfg(), state());
        c.equil = false;
        s.sorb[0] = vec![0.1; N];
        run(&mut c, &mut s);
        let at = |e| s.trace.iter().position(|x| *x == e).unwrap();
        assert!(at(Update(0)) < at(SorbConc(0)));
        assert!(at(SorbConc(0)) < at(MassTranEnd(0)));
        assert!(at(MassTranBegin(0)) < at(Coeff(0, 2)));
    }

    #[test]
    fn nonlinear_species_stops_unconverged_and_resumes_through_the_seam() {
        let (mut c, mut s) = (cfg(), state());
        c.linear = vec![false];
        c.ch_par[0][7] = 0.5;
        c.ch_par[0][8] = 0.8;
        let o = run(&mut c, &mut s);
        assert_eq!(o.pending, Some(0));
        assert!(s.trace.contains(&BanSol(0)));
        assert!(!s.trace.contains(&Update(0)) && !s.trace.contains(&MassTranEnd(0)));
        assert_eq!(s.conc[0], vec![1.0, 0.8, 0.5, 0.2]);
        finish_species(&c, &mut s, 0, 1).unwrap();
        assert_eq!(s.conc[0], s.c_new);
        assert!(s.trace.contains(&Update(0)) && s.trace.contains(&MassTranEnd(0)));
    }

    #[test]
    fn loose_tolerance_reports_convergence() {
        let (mut c, mut s) = (cfg(), state());
        c.linear = vec![false];
        c.ch_par[0][7] = 0.5;
        c.ch_par[0][8] = 0.8;
        c.c_tol_a = 10.0;
        assert_eq!(run(&mut c, &mut s).pending, None);
    }

    #[test]
    fn linear_species_with_unassigned_cmid_is_rejected_without_mutation() {
        let (mut c, mut s) = (cfg(), state());
        s.c_mid = None;
        let before = s.clone();
        assert!(solute_step(&mut c, &mut s).is_err());
        assert_eq!(s.conc, before.conc);
        assert!(s.trace.is_empty());
        // A nonlinear species assigns it.
        let (mut c, mut s) = (cfg(), state());
        c.linear = vec![false];
        c.ch_par[0][7] = 0.5;
        s.c_mid = None;
        run(&mut c, &mut s);
        assert!(s.c_mid.is_some());
    }

    #[test]
    fn unsupported_bacteria_modes_are_rejected() {
        let mut c = cfg();
        c.bact = true;
        c.species_count = 2;
        c.linear = vec![true; 2];
        let mut s = SoluteStepState::blank(N, 2);
        assert!(solute_step(&mut c, &mut s).is_err());
        let mut c = cfg();
        c.bact = true;
        c.dual_por = 1;
        let mut s = state();
        assert!(solute_step(&mut c, &mut s).is_err());
    }

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn matrix_sees_vcorr_subtracted_and_the_closure_sees_it_restored() {
        let (mut c, s0) = (cfg(), state());
        c.ch_par = vec![row(0.5), row(1.0)];
        c.th_sat = vec![0.5, 0.5];
        c.mobile_immobile = vec![false, false];
        c.mat_num = vec![1, 1, 2, 2];
        let mut s = s0;
        s.v_o = vec![0.1, 0.12, 0.14, 0.16];
        // Equal levels give equal corrections, so the final `vCorr` is also Level 1's.
        s.th_n = s.th_o.clone();
        let orig = s.v_o.clone();
        begin_species(&c, &mut s, 0).unwrap();
        solve_pass(&c, &mut s, 0, 1).unwrap();
        assert!(s.v_corr.iter().any(|v| *v != 0.0));
        let ep = s.frame.as_ref().unwrap().endpoint.clone().unwrap();
        for i in 0..N {
            let vc = s.v_corr[i];
            assert_eq!(s.v_o[i].to_bits(), ((orig[i] - vc) + vc).to_bits());
            assert_eq!(ep.old_velocity[i].0, orig[i]);
            assert_eq!(ep.old_matrix_velocity[i].0, orig[i] - vc);
        }
    }

    #[test]
    fn endpoint_rows_get_the_old_and_new_g_pairs_of_their_own_levels() {
        let (c, mut s) = (cfg(), state());
        let mut s = {
            begin_species(&c, &mut s, 0).unwrap();
            s
        };
        solve_pass(&c, &mut s, 0, 1).unwrap();
        let ep = s.frame.as_ref().unwrap().endpoint.clone().unwrap();
        // g0 = xMuL*ThW with sSink = 0: the water contents differ between levels.
        assert!((ep.old_source_g0[0].0 - 0.01 * 0.3).abs() < 1e-7);
        assert!((ep.new_source_g0[0].0 - 0.01 * 0.32).abs() < 1e-7);
        assert_ne!(ep.old_source_g0[0], ep.new_source_g0[0]);
    }

    fn exclusion() -> SizeExclusionPrimitiveState {
        SizeExclusionPrimitiveState {
            nodes: N,
            mobile_residual_water: DefaultReal(0.1),
            mobile_saturated_water: DefaultReal(1.0),
            mobile_n: DefaultReal(2.0),
            mobile_tortuosity: DefaultReal(0.5),
            colloid_exclusion_water_content: DefaultReal(0.2),
            first_node_new_water: DefaultReal(0.6),
            first_node_new_velocity: DefaultReal(-0.25),
        }
    }

    fn uptake() -> RootUptakePrimitiveState {
        RootUptakePrimitiveState {
            species: 1,
            species_count: 1,
            time: DoubleReal(0.0),
            coordinates: dr(&[0.0, 1.0, 2.0, 3.0]),
            root_distribution: vec![DefaultReal(1.0); N],
            root_water_sink: vec![DefaultReal(0.01); N],
            species_concentration: vec![DefaultReal(0.0); N],
            omega_water: DefaultReal(0.8),
            species_root_max_concentration: DefaultReal(1.5),
            active_uptake: false,
            solute_stress_index: DefaultReal(1.0),
            potential_uptake: DefaultReal(0.0),
            michaelis_constant: DefaultReal(1.0),
            minimum_concentration: DefaultReal(0.5),
        }
    }

    fn coupled() -> SoluteStepConfig {
        let mut c = cfg();
        c.bact = true;
        c.water_flow = false;
        c.root_sink = true;
        c.size_exclusion = Some(exclusion());
        c.root_uptake = Some(uptake());
        c
    }

    #[test]
    fn exclusion_then_set_s_snk_then_coeff() {
        let mut c = coupled();
        let mut s = state();
        run(&mut c, &mut s);
        assert_eq!(&s.trace[..3], &[Exclusion, SetSSnk(0), Coeff(0, 1)]);
        // Exclusion rewrote the node arrays and zeroed ChPar(4,1).
        assert_eq!(c.ch_par[0][3], 0.0);
        assert!(s.th_o.iter().all(|t| *t == s.th_o[0]));
    }

    #[test]
    fn exclusion_and_set_s_snk_skip_cases() {
        // lWat true: Exclusion is not called, SetSSnk still is.
        let mut c = coupled();
        c.water_flow = true;
        let mut s = state();
        run(&mut c, &mut s);
        assert_eq!(&s.trace[..2], &[SetSSnk(0), Coeff(0, 1)]);
        // lBact false with SinkF false: neither call.
        let mut c = coupled();
        c.bact = false;
        c.root_sink = false;
        let mut s = state();
        run(&mut c, &mut s);
        assert_eq!(s.trace[0], Coeff(0, 1));
        // SinkF false keeps Exclusion but drops SetSSnk.
        let mut c = coupled();
        c.root_sink = false;
        let mut s = state();
        run(&mut c, &mut s);
        assert_eq!(&s.trace[..2], &[Exclusion, Coeff(0, 1)]);
    }

    #[test]
    fn pulse_reset_follows_the_time_test_and_atmospheric_flag() {
        let (c, mut s) = (cfg(), state());
        s.t = 200.0;
        s.c_top = vec![2.0];
        begin_species(&c, &mut s, 0).unwrap();
        assert_eq!((s.c_top[0], s.c_bot[0]), (0.0, 0.0));
        let mut c = cfg();
        c.atm_bc = true;
        let mut s = state();
        s.t = 200.0;
        begin_species(&c, &mut s, 0).unwrap();
        assert_eq!(s.c_bot[0], 1.0);
    }

    #[test]
    fn flux_concentration_runs_after_the_species_loop() {
        let (mut c, mut s) = (cfg(), state());
        c.conc_type = 2;
        run(&mut c, &mut s);
        assert_eq!(s.trace.last(), Some(&FluxConc));
        assert_eq!(s.trace[s.trace.len() - 2], FluxClose(0));
    }
}
