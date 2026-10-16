//! Nonlinear iteration, convergence, and retry behavior (`SOLUTE.FOR:12-174`, `INPUT.FOR:1348,1368-1371`), task 18.11.1.
//!
//! # Architecture
//!
//! In HYDRUS-1D, linearity `lLinear(jS)` is read in `ChemIn` (`INPUT.FOR:1348-1371`):
//! it starts as `true`, and becomes `false` if `ChPar(8,M)` (Freundlich/Langmuir parameter)
//! deviates from 0 (`abs(ChPar(jjj+8,M)-0.0) > 1.e-12`), or if `ChPar(9,M)` (exponent) deviates
//! from 1 (`abs(ChPar(jjj+9,M)-1.0) > 0.001`), or if bacteria transport (`lBact`) has attachment
//! or detachment rates (`ChPar(jjj+18,M) > 0.0 || ChPar(jjj+15,M) > 0.0`).
//!
//! When `lLinear(jS)` is false, `Solute` executes an iterative loop (label 12):
//! - Each pass performs `Coeff`, `MatSet`, `MassTran(beginning)` (if `Level==1 && Iter==1`),
//!   and `BanSol`.
//! - After `BanSol`, the convergence test evaluates:
//!   `abs(cNew(i) - cTemp(i)) > cTolA + cTolR * Conc(jS, i)`.
//!   If any node exceeds the tolerance, `lConv = false`.
//! - If `!lConv` and `Iter < MaxItC`, execution loops back to label 12 (`goto 12`).
//! - If `!lConv` and `Iter >= MaxItC`, and `dt > dtMin && !lWat`, `dt` is reduced:
//!   `dtOld = dt; dt = max(dt/3.0, dtMin); dtOpt = dt; t = t - dtOld + dt;` and execution
//!   restarts at label 10 (`goto 10`) with the baseline step state rolled back.
//! - If `dt <= dtMin || lWat`, `ierr = 1` is set and the solver terminates.

use crate::solute_fortran_compat::driver::{
    begin_species, finish_species_with_scratch, finish_step, solve_pass_with_scratch,
    SoluteDriverScratch, SoluteStepConfig, SoluteStepState,
};
use crate::solute_fortran_compat::types::KernelError;

/// Configuration parameters governing nonlinear iteration and convergence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NonlinearConfig {
    /// `MaxItC`: maximum number of nonlinear iterations before timestep reduction.
    pub max_it_c: u32,
    /// `dtMin`: minimum allowed timestep for reduction retry.
    pub dt_min: f32,
    /// `cTolA`: absolute concentration tolerance.
    pub c_tol_a: f32,
    /// `cTolR`: relative concentration tolerance.
    pub c_tol_r: f32,
    /// `lWat`: true if water flow is solved simultaneously; prevents dt reduction in solute.
    pub water_flow: bool,
}

impl Default for NonlinearConfig {
    fn default() -> Self {
        Self {
            max_it_c: 10,
            dt_min: 1.0e-5,
            c_tol_a: 0.0,
            c_tol_r: 0.0,
            water_flow: false,
        }
    }
}

/// Convergence check report for one iteration pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ConvergenceCheck {
    /// True if all nodes satisfy the absolute/relative tolerance condition.
    pub converged: bool,
    /// Maximum deviation: `abs(c_new[i] - c_temp[i]) - (c_tol_a + c_tol_r * conc[i])`.
    pub max_residual: f32,
}

/// Check the reader-reachable linearity predicate (`INPUT.FOR:1348,1368-1371`).
///
/// Returns `true` if the species/material combination is linear, `false` if nonlinear.
///
/// Pinned formulas:
/// ```fortran
///   lLinear(jj)=.true.
///   do 12 M=1,NMat
///     ...
///     if(abs(ChPar(jjj+8,M)-0.0).gt.1.e-12.or.
/// !     abs(ChPar(jjj+9,M)-1.0).gt.0.001) lLinear(jj)=.false.
///     if(lBact.and.(ChPar(jjj+18,M).gt.0..or.ChPar(jjj+15,M).gt.0.))
/// !                                       lLinear(jj)=.false.
/// 12 continue
/// ```
/// Note: in 0-indexed `ch_par_row`:
/// - `ChPar(8)` is index 7
/// - `ChPar(9)` is index 8
/// - `ChPar(15)` is index 14
/// - `ChPar(18)` is index 17
pub(crate) fn check_linear_predicate(ch_par_row: &[f32], is_bact: bool) -> bool {
    if ch_par_row.len() < 9 {
        return true;
    }
    let p8 = ch_par_row[7];
    let p9 = ch_par_row[8];
    if (p8 - 0.0_f32).abs() > 1.0e-12 || (p9 - 1.0_f32).abs() > 0.001 {
        return false;
    }
    if is_bact && ch_par_row.len() >= 18 {
        let p15 = ch_par_row[14];
        let p18 = ch_par_row[17];
        if p18 > 0.0 || p15 > 0.0 {
            return false;
        }
    }
    true
}

/// Evaluate nonlinear convergence across all nodes (`SOLUTE.FOR:153-158`).
///
/// ```fortran
///   lConv=.true.
///   do 16 i=1,N
///     ...
///     if(abs(cNew(i)-cTemp(i)).gt.cTolA+cTolR*Conc(jS,i))
///  !    lConv=.false.
/// 16 continue
/// ```
pub(crate) fn evaluate_convergence(
    c_new: &[f32],
    c_temp: &[f32],
    conc: &[f32],
    c_tol_a: f32,
    c_tol_r: f32,
) -> ConvergenceCheck {
    let mut converged = true;
    let mut max_residual = 0.0_f32;
    for i in 0..c_new.len() {
        let diff = (c_new[i] - c_temp[i]).abs();
        let tol = c_tol_a + c_tol_r * conc[i];
        let residual = diff - tol;
        if residual > max_residual {
            max_residual = residual;
        }
        if diff > tol {
            converged = false;
        }
    }
    ConvergenceCheck {
        converged,
        max_residual,
    }
}

/// Outcome of attempting iteration for a single species.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SpeciesIterationOutcome {
    /// Converged within `iterations`.
    Converged { iterations: u32 },
    /// Did not converge within `max_it_c`.
    Unconverged { iterations: u32 },
}

/// Execute nonlinear iteration for a single species `js` (`SOLUTE.FOR:12-164`).
///
/// If `cfg.linear[js]` is true, this executes exactly one iteration pass.
/// If `!cfg.linear[js]`, it iterates up to `nonlin_cfg.max_it_c`.
pub(crate) fn step_nonlinear_species(
    cfg: &SoluteStepConfig,
    st: &mut SoluteStepState,
    js: usize,
    nonlin_cfg: &NonlinearConfig,
) -> Result<SpeciesIterationOutcome, KernelError> {
    let mut scratch = SoluteDriverScratch::new(cfg.x.len(), cfg.species_count, cfg.th_sat.len());
    step_nonlinear_species_with_scratch(cfg, st, js, nonlin_cfg, &mut scratch)
}

/// Execute nonlinear iteration for a single species `js` with caller-provided scratch buffers (`SOLUTE.FOR:12-164`).
pub(crate) fn step_nonlinear_species_with_scratch(
    cfg: &SoluteStepConfig,
    st: &mut SoluteStepState,
    js: usize,
    nonlin_cfg: &NonlinearConfig,
    scratch: &mut SoluteDriverScratch,
) -> Result<SpeciesIterationOutcome, KernelError> {
    let mut iter: u32 = 1;
    let max_iter = if cfg.linear[js] {
        1
    } else {
        nonlin_cfg.max_it_c.max(1)
    };

    loop {
        let pass = solve_pass_with_scratch(cfg, st, js, iter, scratch)?;
        st.iter_c = st.iter_c.max(iter);

        if cfg.linear[js] || pass.converged {
            finish_species_with_scratch(cfg, st, js, iter, scratch)?;
            return Ok(SpeciesIterationOutcome::Converged { iterations: iter });
        }

        if iter >= max_iter {
            return Ok(SpeciesIterationOutcome::Unconverged { iterations: iter });
        }

        iter += 1;
    }
}

/// Outcome of the overall solute iteration solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SoluteIterationReport {
    /// Total number of timestep reductions performed.
    pub retries: u32,
    /// Final accepted timestep `dt`.
    pub dt: f32,
    /// Final current time `t`.
    pub t: f64,
    /// Maximum iterations taken by any species.
    pub max_iter: u32,
}

/// Execute the full solute step with nonlinear iteration, timestep reduction retry,
/// and rollback/commit sequencing matching `SOLUTE.FOR:10-226`.
pub(crate) fn solve_solute_with_nonlinear(
    cfg: &mut SoluteStepConfig,
    st: &mut SoluteStepState,
    nonlin_cfg: &NonlinearConfig,
) -> Result<SoluteIterationReport, KernelError> {
    let mut scratch = SoluteDriverScratch::new(cfg.x.len(), cfg.species_count, cfg.th_sat.len());
    solve_solute_with_nonlinear_with_scratch(cfg, st, nonlin_cfg, &mut scratch)
}

/// Execute the full solute step with nonlinear iteration, timestep reduction retry,
/// and rollback/commit sequencing matching `SOLUTE.FOR:10-226`, using preallocated scratch buffers.
pub(crate) fn solve_solute_with_nonlinear_with_scratch(
    cfg: &mut SoluteStepConfig,
    st: &mut SoluteStepState,
    nonlin_cfg: &NonlinearConfig,
    scratch: &mut SoluteDriverScratch,
) -> Result<SoluteIterationReport, KernelError> {
    cfg.c_tol_a = nonlin_cfg.c_tol_a;
    cfg.c_tol_r = nonlin_cfg.c_tol_r;

    // Snapshot state at label 10 before species loop begins into scratch.backup_state
    scratch.backup_state.copy_from(st);
    let baseline_dt = st.dt;
    let baseline_t = st.t;

    let mut current_dt = baseline_dt;
    let mut current_t = baseline_t;
    let mut retries: u32 = 0;

    'retry_loop: loop {
        // Reset state to baseline snapshot for this try without heap allocations
        st.copy_from(&scratch.backup_state);
        st.dt = current_dt;
        st.t = current_t;
        st.iter_c = 1;
        st.ierr = 0;

        let mut max_species_iter: u32 = 1;

        for js in 0..cfg.species_count {
            begin_species(cfg, st, js)?;

            let outcome = step_nonlinear_species_with_scratch(cfg, st, js, nonlin_cfg, scratch)?;
            match outcome {
                SpeciesIterationOutcome::Converged { iterations } => {
                    max_species_iter = max_species_iter.max(iterations);
                }
                SpeciesIterationOutcome::Unconverged { .. } => {
                    // Check if timestep reduction retry is permitted (SOLUTE.FOR:164-173)
                    if current_dt > nonlin_cfg.dt_min && !nonlin_cfg.water_flow {
                        let dt_old = current_dt;
                        current_dt = (current_dt / 3.0_f32).max(nonlin_cfg.dt_min);
                        current_t = current_t - (dt_old as f64) + (current_dt as f64);
                        retries += 1;
                        continue 'retry_loop;
                    }
                    // Cannot reduce timestep: flag ierr = 1 and error
                    st.ierr = 1;
                    return Err(
                        "Solute transport failed to converge within maximum iterations or minimum timestep",
                    );
                }
            }
        }

        // Finish step (flux concentrations etc.) after all species converge
        finish_step(cfg, st)?;

        return Ok(SoluteIterationReport {
            retries,
            dt: current_dt,
            t: current_t,
            max_iter: max_species_iter,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solute_fortran_compat::driver::{DriverEvent, SoluteStepConfig, SoluteStepState};

    const N: usize = 4;

    fn row(kd: f32) -> Vec<f32> {
        let mut r = vec![0.0_f32; 36];
        r[0] = 1.0;
        r[1] = 0.05;
        r[2] = 1.0;
        r[3] = 0.0;
        r[6] = kd;
        r[7] = 0.5; // Langmuir/Freundlich parameter
        r[8] = 0.8; // Exponent != 1.0
        r
    }

    fn base_cfg() -> SoluteStepConfig {
        SoluteStepConfig {
            x: vec![0.0, 1.0, 2.0, 3.0],
            mat_num: vec![1; N],
            th_sat: vec![0.5],
            mobile_immobile: vec![false],
            ch_par: vec![row(0.0)],
            t_dep: vec![0.0; 36],
            species_count: 1,
            linear: vec![false],
            equil: true,
            upwind: false,
            art_disp: false,
            tort: false,
            i_tort: 0,
            bact: false,
            filtr: false,
            dual_neq: false,
            water_flow: false,
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

    fn base_state() -> SoluteStepState {
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

    #[test]
    fn test_check_linear_predicate_defaults_and_triggers() {
        // Linear default: ChPar(8) = 0.0, ChPar(9) = 1.0, no bact
        let mut r = vec![0.0_f32; 20];
        r[7] = 0.0;
        r[8] = 1.0;
        assert!(check_linear_predicate(&r, false));

        // Freundlich / Langmuir parameter ChPar(8) > 0 triggers nonlinear
        r[7] = 0.5;
        assert!(!check_linear_predicate(&r, false));
        r[7] = 0.0;

        // Exponent ChPar(9) != 1.0 triggers nonlinear
        r[8] = 0.9;
        assert!(!check_linear_predicate(&r, false));
        r[8] = 1.0;

        // Bacteria with attachment rate ChPar(15) > 0 triggers nonlinear
        r[14] = 0.1;
        assert!(!check_linear_predicate(&r, true));
        assert!(check_linear_predicate(&r, false)); // only if is_bact
        r[14] = 0.0;

        // Bacteria with detachment rate ChPar(18) > 0 triggers nonlinear
        r[17] = 0.05;
        assert!(!check_linear_predicate(&r, true));
        assert!(check_linear_predicate(&r, false));
    }

    #[test]
    fn test_evaluate_convergence_tolerance_check() {
        let c_new = vec![1.001, 0.8, 0.5, 0.2];
        let c_temp = vec![1.0, 0.8, 0.5, 0.2];
        let conc = vec![1.0, 0.8, 0.5, 0.2];

        // Absolute tolerance satisfies
        let check1 = evaluate_convergence(&c_new, &c_temp, &conc, 0.01, 0.0);
        assert!(check1.converged);

        // Absolute tolerance too tight
        let check2 = evaluate_convergence(&c_new, &c_temp, &conc, 0.0001, 0.0);
        assert!(!check2.converged);

        // Relative tolerance satisfies
        let check3 = evaluate_convergence(&c_new, &c_temp, &conc, 0.0, 0.01);
        assert!(check3.converged);

        // Relative tolerance too tight
        let check4 = evaluate_convergence(&c_new, &c_temp, &conc, 0.0, 0.0001);
        assert!(!check4.converged);
    }

    #[test]
    fn test_step_nonlinear_species_linear_single_iteration() {
        let mut cfg = base_cfg();
        cfg.linear = vec![true];
        let mut st = base_state();

        begin_species(&cfg, &mut st, 0).unwrap();
        let nonlin_cfg = NonlinearConfig {
            max_it_c: 10,
            ..Default::default()
        };
        let outcome = step_nonlinear_species(&cfg, &mut st, 0, &nonlin_cfg).unwrap();
        assert_eq!(
            outcome,
            SpeciesIterationOutcome::Converged { iterations: 1 }
        );
        assert_eq!(st.iter_c, 1);
        assert!(st.trace.contains(&DriverEvent::Update(0)));
        assert!(st.trace.contains(&DriverEvent::MassTranEnd(0)));
    }

    #[test]
    fn test_step_nonlinear_species_converges_with_tolerance() {
        let mut cfg = base_cfg();
        cfg.linear = vec![false];
        let mut st = base_state();

        begin_species(&cfg, &mut st, 0).unwrap();
        // Set loose tolerance so it converges on the first pass
        cfg.c_tol_a = 5.0;
        let nonlin_cfg = NonlinearConfig {
            max_it_c: 5,
            c_tol_a: 5.0,
            ..Default::default()
        };
        let outcome = step_nonlinear_species(&cfg, &mut st, 0, &nonlin_cfg).unwrap();
        assert_eq!(
            outcome,
            SpeciesIterationOutcome::Converged { iterations: 1 }
        );
        assert_eq!(st.conc[0], st.c_new);
    }

    #[test]
    fn test_step_nonlinear_species_unconverged_stops_at_max_it() {
        let mut cfg = base_cfg();
        cfg.linear = vec![false];
        cfg.c_tol_a = 0.0;
        cfg.c_tol_r = 0.0;
        let mut st = base_state();

        begin_species(&cfg, &mut st, 0).unwrap();
        let nonlin_cfg = NonlinearConfig {
            max_it_c: 1,
            c_tol_a: 0.0,
            c_tol_r: 0.0,
            ..Default::default()
        };
        let outcome = step_nonlinear_species(&cfg, &mut st, 0, &nonlin_cfg).unwrap();
        assert_eq!(
            outcome,
            SpeciesIterationOutcome::Unconverged { iterations: 1 }
        );
        assert_eq!(st.iter_c, 1);
    }

    #[test]
    fn test_solve_solute_with_nonlinear_timestep_reduction_and_retry() {
        let mut cfg = base_cfg();
        cfg.linear = vec![false];
        let mut st = base_state();
        let initial_dt = 0.09;
        st.dt = initial_dt;
        st.t = 1.0;

        let nonlin_cfg = NonlinearConfig {
            max_it_c: 1,
            dt_min: 0.001,
            c_tol_a: 0.0,
            c_tol_r: 0.0,
            water_flow: false,
        };

        // When max_it_c is 1 and c_tol is 0, it won't converge on pass 1,
        // so it will reduce dt repeatedly down to dt_min
        let err = solve_solute_with_nonlinear(&mut cfg, &mut st, &nonlin_cfg).unwrap_err();
        assert_eq!(
            err,
            "Solute transport failed to converge within maximum iterations or minimum timestep"
        );
        assert_eq!(st.ierr, 1);
    }

    #[test]
    fn test_solve_solute_with_nonlinear_water_flow_halts_without_dt_reduction() {
        let mut cfg = base_cfg();
        cfg.linear = vec![false];
        cfg.water_flow = true; // simultaneous water flow
        let mut st = base_state();
        st.dt = 0.1;

        let nonlin_cfg = NonlinearConfig {
            max_it_c: 1,
            dt_min: 0.001,
            c_tol_a: 0.0,
            c_tol_r: 0.0,
            water_flow: true,
        };

        let err = solve_solute_with_nonlinear(&mut cfg, &mut st, &nonlin_cfg).unwrap_err();
        assert_eq!(
            err,
            "Solute transport failed to converge within maximum iterations or minimum timestep"
        );
        assert_eq!(st.ierr, 1);
        // Did not reduce dt
        assert_eq!(st.dt, 0.1);
    }

    #[test]
    fn test_solve_solute_with_nonlinear_multi_species_success() {
        let mut cfg = base_cfg();
        cfg.species_count = 2;
        cfg.linear = vec![true, false];
        cfg.ch_par = vec![row(0.0)];
        cfg.t_dep = vec![0.0; 36];

        let mut st = SoluteStepState::blank(N, 2);
        st.conc[0] = vec![1.0, 0.8, 0.5, 0.2];
        st.conc[1] = vec![0.5, 0.4, 0.2, 0.1];
        st.th_o = vec![0.3; N];
        st.th_n = vec![0.32; N];
        st.v_o = vec![0.1; N];
        st.v_n = vec![0.1; N];
        st.temp_o = vec![20.0; N];
        st.temp_n = vec![20.0; N];
        st.c_bot = vec![1.0, 0.5];
        st.t = 1.0;
        st.dt = 0.1;
        st.c_mid = Some(0.0);

        let nonlin_cfg = NonlinearConfig {
            max_it_c: 5,
            c_tol_a: 5.0, // loose tolerance for convergence
            ..Default::default()
        };

        let report = solve_solute_with_nonlinear(&mut cfg, &mut st, &nonlin_cfg).unwrap();
        assert_eq!(report.retries, 0);
        assert_eq!(report.dt, 0.1);
        assert!(st.trace.contains(&DriverEvent::Update(0)));
        assert!(st.trace.contains(&DriverEvent::Update(1)));
    }

    #[test]
    fn test_solve_solute_with_nonlinear_with_scratch_bit_identical_to_standard() {
        let mut cfg1 = base_cfg();
        cfg1.linear = vec![false];
        let mut st1 = base_state();
        let initial_dt = 0.09;
        st1.dt = initial_dt;
        st1.t = 1.0;

        let mut cfg2 = cfg1.clone();
        let mut st2 = st1.clone();

        let nonlin_cfg = NonlinearConfig {
            max_it_c: 1,
            dt_min: 0.001,
            c_tol_a: 0.0,
            c_tol_r: 0.0,
            water_flow: false,
        };

        let err1 = solve_solute_with_nonlinear(&mut cfg1, &mut st1, &nonlin_cfg).unwrap_err();
        let mut scratch =
            SoluteDriverScratch::new(cfg2.x.len(), cfg2.species_count, cfg2.th_sat.len());
        let err2 = solve_solute_with_nonlinear_with_scratch(
            &mut cfg2,
            &mut st2,
            &nonlin_cfg,
            &mut scratch,
        )
        .unwrap_err();

        assert_eq!(err1, err2);
        assert_eq!(st1.ierr, st2.ierr);
        assert_eq!(st1.dt, st2.dt);
        assert_eq!(st1.t, st2.t);
        assert_eq!(st1.conc, st2.conc);
        assert_eq!(st1.sorb, st2.sorb);
        assert_eq!(st1.c_new, st2.c_new);
        assert_eq!(st1.trace, st2.trace);
    }
}
