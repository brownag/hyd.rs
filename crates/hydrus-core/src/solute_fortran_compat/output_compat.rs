//! Source-precision `OUTPUT.FOR` table formatters.
//!
//! Generates pure in-memory `String` representations of:
//! - `PROFILE.OUT` (`Profil` in `INPUT.FOR:1256-1283` and static grid table)
//! - `NOD_INF.OUT` and `NOD_INF_V.OUT` (`NodOut` in `OUTPUT.FOR:720-927`)
//! - `solute1..NS.out` (`TLInf` in `OUTPUT.FOR:3-292`, `:205-225`)
//! - `OBS_NODE.OUT` (`ObsNod` in `OUTPUT.FOR:931-1075` / `1080-1107`)
//! - `BALANCE.OUT` (`SubReg` in `OUTPUT.FOR:325-716`)
//!
//! Formatting strictly preserves Fortran column widths, header labels,
//! scientific exponent representations (`0.xxxxxE+ee`), and trailing `end` markers
//! required by `tools/solute_parity_normalizers.py`.

use std::fmt::Write;

/// Format a float in standard Fortran scientific notation `0.xxxxxE+ee` (or ` -0.xxxxxE+ee`).
///
/// `w` is the total field width, right-aligned.
/// `d` is the number of digits after the decimal point in the mantissa `0.xxxxx`.
pub fn format_fortran_e(val: f64, w: usize, d: usize) -> String {
    if !val.is_finite() {
        return format!(
            "{:>w$}",
            if val.is_nan() { "NaN" } else { "Infinity" },
            w = w
        );
    }
    if val == 0.0 {
        let zeros = "0".repeat(d);
        let s = format!(" 0.{}E+00", zeros);
        return format!("{:>w$}", s, w = w);
    }
    let sign = if val < 0.0 { "-" } else { " " };
    let abs_v = val.abs();
    // Exact decimal conversion of the binary value, rounded to nearest with
    // ties to even, as libgfortran does when `ROUND=` is unspecified (e.g. the
    // `e13.5` write at `OUTPUT.FOR:209` prints `49.0625` as `0.49062E+02`;
    // probe evidence in `planning/master/WATER_TRANSIENT_DIAGNOSIS_21R_TEMPVAPOR.md`).
    // Rust's `{:.Ne}` gives `d` significant digits `x.xxxxe<k>` and carries into
    // the exponent; Fortran's `0.xxxxxE<k+1>` is the same digits re-based.
    let sci = format!("{:.prec$e}", abs_v, prec = d.max(1) - 1);
    let (digits, rust_exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp_val = rust_exp.parse::<i32>().unwrap_or(0) + 1;
    let man_str = format!("0.{}", digits.replace('.', ""));
    let exp_sign = if exp_val >= 0 { "+" } else { "-" };
    let exp_str = format!("{}{:02}", exp_sign, exp_val.abs());
    let s = format!("{}{}E{}", sign, man_str, exp_str);
    format!("{:>w$}", s, w = w)
}

/// Format a float in Fortran `f{w}.{d}` format.
pub fn format_fortran_f(val: f64, w: usize, d: usize) -> String {
    if !val.is_finite() {
        return format!(
            "{:>w$}",
            if val.is_nan() { "NaN" } else { "Infinity" },
            w = w
        );
    }
    let s = format!("{:.precision$}", val, precision = d);
    format!("{:>w$}", s, w = w)
}

/// Record for a single node row in `PROFILE.OUT` (`INPUT.FOR:1256-1283`).
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileRow {
    /// 1-based top-to-bottom node index (`N - i + 1`).
    pub n: usize,
    /// Depth below surface (`xSurf - x(i)`).
    pub depth: f64,
    /// Residual moisture content `thr(M)`.
    pub thr: f64,
    /// Saturated moisture content `thr(M) + ATh(i)*(thS(M) - thr(M))`.
    pub ths: f64,
    /// Air entry / scaling pressure head `hS(M) * Ah(i)`.
    pub hs: f64,
    /// Saturated hydraulic conductivity `ConS(M) * AK(i)`.
    pub ks: f64,
    /// Normalized conductivity `ConS(M) * AK(i) / ConSN`.
    pub ks_kstop: f64,
    /// Root water uptake factor `Beta(i)`.
    pub beta: f64,
    /// Pressure head scaling factor `Ah(i)`.
    pub ah: f64,
    /// Hydraulic conductivity scaling factor `AK(i)`.
    pub ak: f64,
    /// Water content scaling factor `ATh(i)`.
    pub ath: f64,
}

/// Format `PROFILE.OUT` matching `INPUT.FOR:1256-1283`.
pub fn format_profile_out(rows: &[ProfileRow]) -> String {
    let mut out = String::new();
    writeln!(out).unwrap();
    writeln!(
        out,
        "    n      depth     THr       THs       hs       Ks        Ks/KsTop     Beta      Ah        AK        ATh"
    )
    .unwrap();
    writeln!(out).unwrap();

    for r in rows {
        // 120 format(i5,f10.2,2f10.3,f10.1,e12.3,5f10.3)
        let n_str = format!("{:>5}", r.n);
        let depth_str = format_fortran_f(r.depth, 10, 2);
        let thr_str = format_fortran_f(r.thr, 10, 3);
        let ths_str = format_fortran_f(r.ths, 10, 3);
        let hs_str = format_fortran_f(r.hs, 10, 1);
        let ks_str = format_fortran_e(r.ks, 12, 3);
        let ks_ratio_str = format_fortran_f(r.ks_kstop, 10, 3);
        let beta_str = format_fortran_f(r.beta, 10, 3);
        let ah_str = format_fortran_f(r.ah, 10, 3);
        let ak_str = format_fortran_f(r.ak, 10, 3);
        let ath_str = format_fortran_f(r.ath, 10, 3);

        writeln!(
            out,
            "{}{}{}{}{}{}{}{}{}{}{}",
            n_str,
            depth_str,
            thr_str,
            ths_str,
            hs_str,
            ks_str,
            ks_ratio_str,
            beta_str,
            ah_str,
            ak_str,
            ath_str
        )
        .unwrap();
    }
    writeln!(out, "end").unwrap();
    out
}

/// Nodal entry for `NOD_INF.OUT`.
#[derive(Clone, Debug, PartialEq)]
pub struct NodInfRow {
    /// 1-based top-to-bottom node index (`N - i + 1`).
    pub node: usize,
    /// Distance from surface `x(i) - xS`.
    pub depth: f64,
    /// Pressure head `hNew(i)`.
    pub head: f64,
    /// Moisture content `thN(i)`.
    pub moisture: f64,
    /// Hydraulic conductivity `Con(i)` (or `SinkIm(i)` in dual-porosity).
    pub k: f64,
    /// Capacity `Cap(i)` (or `ThNewIm(i)` in dual-porosity).
    pub c: f64,
    /// Total flux `vi + vVi`.
    pub flux: f64,
    /// Sink term `Sink(i)` (or `STrans(i)` in dual-porosity).
    pub sink: f64,
    /// Nodal index code `Kappa(i)`.
    pub kappa: i32,
    /// Normalized velocity `vi / ConSN`.
    pub v_kstop: f64,
    /// Temperature `Temp(i)`.
    pub temp: f64,
    /// Dissolved concentrations `Conc(1..NS, i)`.
    pub conc: Vec<f64>,
    /// Sorbed concentrations `Sorb(1..NS, i)`.
    pub sorb: Vec<f64>,
    /// Secondary sorbed concentrations `Sorb2(1..NS, i)` (dual-nonequilibrium or bacteria).
    pub sorb2: Vec<f64>,
}

/// Block of nodal output for a single print time step.
#[derive(Clone, Debug, PartialEq)]
pub struct NodInfBlock {
    pub time: f64,
    pub num_species: usize,
    pub dual_porosity: bool,
    pub rows: Vec<NodInfRow>,
}

/// Format `NOD_INF.OUT` matching `OUTPUT.FOR:720-927`.
pub fn format_nod_inf_out(blocks: &[NodInfBlock]) -> String {
    let mut out = String::new();
    for block in blocks {
        writeln!(out).unwrap();
        if block.time < 99999999.0 {
            writeln!(out, " Time:{}", format_fortran_f(block.time, 14, 4)).unwrap();
        } else {
            writeln!(out, " Time:{}", format_fortran_e(block.time, 15, 8)).unwrap();
        }
        writeln!(out).unwrap();

        if block.num_species == 0 {
            if !block.dual_porosity {
                writeln!(out, " Node      Depth      Head Moisture       K          C         Flux        Sink         Kappa   v/KsTop   Temp").unwrap();
                writeln!(out, "           [L]        [L]    [-]        [L/T]      [1/L]      [L/T]        [1/T]         [-]      [-]      [C]").unwrap();
            } else {
                writeln!(out, " Node      Depth      Head Moisture    WTrans    Im.Moist.     Flux       STrans        Kappa   v/KsTop   Temp").unwrap();
                writeln!(out, "           [L]        [L]    [-]        [1/T]       [-]       [L/T]     [M/L*3/T]        [-]      [-]      [C]").unwrap();
            }
        } else if !block.dual_porosity {
            writeln!(out, " Node      Depth      Head Moisture       K          C         Flux        Sink         Kappa   v/KsTop   Temp   Conc(1..NS) Sorb(1...NS)").unwrap();
            writeln!(out, "           [L]        [L]    [-]        [L/T]      [1/L]      [L/T]        [1/T]         [-]      [-]      [C]      [M/L*3]").unwrap();
        } else {
            writeln!(out, " Node      Depth      Head Moisture    WTrans    Im.Moist.     Flux       STrans        Kappa   v/KsTop   Temp   Conc(1..NS) Sorb(1...NS)").unwrap();
            writeln!(out, "           [L]        [L]    [-]        [1/T]       [-]       [L/T]     [M/L*3/T]        [-]      [-]      [C]      [M/L*3]").unwrap();
        }
        writeln!(out).unwrap();

        for r in &block.rows {
            // format 120 vs 130:
            // 120 format(i4,1x,f10.4,1x,f11.3,1x,f6.4,1x,4e12.4,i8,1x,e11.3,f8.2,30e12.4)
            // 130 format(i4,1x,f10.4,1x,e11.4,1x,f6.4,1x,4e12.4,i8,2x,f10.3,f8.2,30e12.4)
            let node_str = format!("{:>4}", r.node);
            let depth_str = format_fortran_f(r.depth, 10, 4);
            let head_str = if r.head > -9.9e+05 {
                format_fortran_f(r.head, 11, 3)
            } else {
                format_fortran_e(r.head, 11, 4)
            };
            let moist_str = format_fortran_f(r.moisture, 6, 4);
            let k_str = format_fortran_e(r.k, 12, 4);
            let c_str = format_fortran_e(r.c, 12, 4);
            let flux_str = format_fortran_e(r.flux, 12, 4);
            let sink_str = format_fortran_e(r.sink, 12, 4);
            let kappa_str = format!("{:>8}", r.kappa);
            let v_ratio_str = if r.head > -9.9e+05 {
                format!(" {}", format_fortran_e(r.v_kstop, 11, 3))
            } else {
                format!("  {}", format_fortran_f(r.v_kstop, 10, 3))
            };
            let temp_str = format_fortran_f(r.temp, 8, 2);

            let mut conc_sorb_str = String::new();
            for c_val in &r.conc {
                conc_sorb_str.push_str(&format_fortran_e(*c_val, 12, 4));
            }
            for s_val in &r.sorb {
                conc_sorb_str.push_str(&format_fortran_e(*s_val, 12, 4));
            }
            for s2_val in &r.sorb2 {
                conc_sorb_str.push_str(&format_fortran_e(*s2_val, 12, 4));
            }

            writeln!(
                out,
                "{} {}{} {} {}{}{}{}{}{}{}{}",
                node_str,
                depth_str,
                head_str,
                moist_str,
                k_str,
                c_str,
                flux_str,
                sink_str,
                kappa_str,
                v_ratio_str,
                temp_str,
                conc_sorb_str
            )
            .unwrap();
        }
        writeln!(out, "end").unwrap();
    }
    out
}

/// Nodal entry for vapor flux table `NOD_INF_V.OUT`.
#[derive(Clone, Debug, PartialEq)]
pub struct NodInfVRow {
    pub node: usize,
    pub depth: f64,
    pub con: f64,
    pub con_lt: f64,
    pub con_vh: f64,
    pub con_vt: f64,
    pub v_liquid: f64,
    pub v_vapor: f64,
    pub v_total: f64,
    pub v_vap_iso: f64,
    pub v_vap_therm: f64,
}

/// Block of vapor nodal output.
#[derive(Clone, Debug, PartialEq)]
pub struct NodInfVBlock {
    pub time: f64,
    pub rows: Vec<NodInfVRow>,
}

/// Format `NOD_INF_V.OUT` matching `OUTPUT.FOR:758, 815, 916-926`.
pub fn format_nod_inf_v_out(blocks: &[NodInfVBlock]) -> String {
    let mut out = String::new();
    for block in blocks {
        writeln!(out).unwrap();
        if block.time < 99999999.0 {
            writeln!(out, " Time:{}", format_fortran_f(block.time, 14, 4)).unwrap();
        } else {
            writeln!(out, " Time:{}", format_fortran_e(block.time, 15, 8)).unwrap();
        }
        writeln!(out).unwrap();
        writeln!(
            out,
            " Node    Depth        Con        ConLT        ConVh        ConVT       vLiquid       vVapor       vTotal       vVapIso     vVapTerm"
        )
        .unwrap();
        writeln!(
            out,
            "          [L]        [L/T]     [L2/K/T]       [L/T]      [L2/K/T]       [L/T]         [L/T]        [L/T]        [L/T]        [L/T]"
        )
        .unwrap();
        writeln!(out).unwrap();

        for r in &block.rows {
            // 140 format(i4,1x,f10.4,1x,9e13.5)
            let node_str = format!("{:>4}", r.node);
            let depth_str = format_fortran_f(r.depth, 10, 4);
            let c_str = format_fortran_e(r.con, 13, 5);
            let clt_str = format_fortran_e(r.con_lt, 13, 5);
            let cvh_str = format_fortran_e(r.con_vh, 13, 5);
            let cvt_str = format_fortran_e(r.con_vt, 13, 5);
            let vl_str = format_fortran_e(r.v_liquid, 13, 5);
            let vv_str = format_fortran_e(r.v_vapor, 13, 5);
            let vt_str = format_fortran_e(r.v_total, 13, 5);
            let vi_str = format_fortran_e(r.v_vap_iso, 13, 5);
            let vterm_str = format_fortran_e(r.v_vap_therm, 13, 5);

            writeln!(
                out,
                "{} {}{}{}{}{}{}{}{}{}{}",
                node_str,
                depth_str,
                c_str,
                clt_str,
                cvh_str,
                cvt_str,
                vl_str,
                vv_str,
                vt_str,
                vi_str,
                vterm_str
            )
            .unwrap();
        }
        writeln!(out, "end").unwrap();
    }
    out
}

/// Flux observation pair `(vNew(Node(j))*Conc(jS,Node(j)), CumCh(6+j,jS))` for `solute1..NS.out`.
#[derive(Clone, Debug, PartialEq)]
pub struct SoluteObsPair {
    pub flux: f64,
    pub cum_flux: f64,
}

/// Per-time-step record for `solute1..NS.out`.
#[derive(Clone, Debug, PartialEq)]
pub struct SoluteRow {
    pub time: f64,
    pub cv_top: f64,
    pub cv_bot: f64,
    pub sum_cv_top: f64,
    pub sum_cv_bot: f64,
    /// Cumulative `CumCh(3,jS)` (OUTPUT.FOR:190 column 6, printed under `cvCh0`).
    pub cv_ch0: f64,
    /// Cumulative `CumCh(4,jS)` (OUTPUT.FOR:190 column 7, printed under `cvCh1`).
    pub cv_ch1: f64,
    pub c_top: f64,
    pub c_root: f64,
    pub c_bot: f64,
    pub cv_root: f64,
    pub sum_cv_root: f64,
    /// Cumulative `CumCh(6,jS)`, the `cvChIm` sum printed under `Sum(cvNEql)`.
    pub sum_cv_ch1: f64,
    pub t_level: i32,
    pub c_gwl: f64,
    pub c_runoff: f64,
    pub sum_c_runoff: f64,
    /// When `lFlux` is active, up to 3 observation node pairs (flux and cumulative flux).
    pub obs_fluxes: Vec<SoluteObsPair>,
}

/// Format header and records for `solute1..NS.out` matching `OUTPUT.FOR:160, 190, 191`.
pub fn format_solute_out(rows: &[SoluteRow]) -> String {
    let mut out = String::new();
    writeln!(
        out,
        " All solute fluxes and cumulative solute fluxes are positive into the region"
    )
    .unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "       Time         cvTop        cvBot      Sum(cvTop)   Sum(cvBot)     cvCh0        cvCh1         cTop        cRoot         cBot        cvRoot    Sum(cvRoot)  Sum(cvNEql) TLevel      cGWL        cRunOff   Sum(cRunOff)    (cv(i),    Sum(cv(i)), i=1,NObs)"
    )
    .unwrap();
    writeln!(
        out,
        "        [T]        [M/L2/T]     [M/L2/T]      [M/L2]       [M/L2]       [M/L2]      [M/L2]        [M/L3]      [M/L3]        [M/L3]      [M/L2/T]      [M/L2]       [M/L2]              [M/L3]        [M/L2]      [M/L3]      [M/L2/T]      [M/L2]"
    )
    .unwrap();

    for r in rows {
        // Fortran lines 208-224:
        // format 190: f14.4,12e13.5,i8,e13.5,8e13.5
        // format 191: e15.8,12e13.5,i8,e13.5,8e13.5
        let t_str = if r.time < 99999999.0 {
            format_fortran_f(r.time, 14, 4)
        } else {
            format_fortran_e(r.time, 15, 8)
        };
        // In Fortran: -cvTop(jS) is printed!
        let neg_cv_top = -r.cv_top;

        let col2 = format_fortran_e(neg_cv_top, 13, 5);
        let col3 = format_fortran_e(r.cv_bot, 13, 5);
        let col4 = format_fortran_e(r.sum_cv_top, 13, 5);
        let col5 = format_fortran_e(r.sum_cv_bot, 13, 5);
        let col6 = format_fortran_e(r.cv_ch0, 13, 5);
        let col7 = format_fortran_e(r.cv_ch1, 13, 5);
        let col8 = format_fortran_e(r.c_top, 13, 5);
        let col9 = format_fortran_e(r.c_root, 13, 5);
        let col10 = format_fortran_e(r.c_bot, 13, 5);
        let col11 = format_fortran_e(r.cv_root, 13, 5);
        let col12 = format_fortran_e(r.sum_cv_root, 13, 5);
        let col13 = format_fortran_e(r.sum_cv_ch1, 13, 5);
        let col14 = format!("{:>8}", r.t_level);
        let col15 = format_fortran_e(r.c_gwl, 13, 5);
        let col16 = format_fortran_e(r.c_runoff, 13, 5);
        let col17 = format_fortran_e(r.sum_c_runoff, 13, 5);

        let mut line = format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
            t_str,
            col2,
            col3,
            col4,
            col5,
            col6,
            col7,
            col8,
            col9,
            col10,
            col11,
            col12,
            col13,
            col14,
            col15,
            col16,
            col17
        );

        for obs in r.obs_fluxes.iter().take(3) {
            line.push_str(&format_fortran_e(obs.flux, 13, 5));
            line.push_str(&format_fortran_e(obs.cum_flux, 13, 5));
        }

        writeln!(out, "{}", line).unwrap();
    }
    writeln!(out, "end").unwrap();
    out
}

/// Entry for one observation node at a single point in time.
#[derive(Clone, Debug, PartialEq)]
pub struct ObsNodeEntry {
    pub head: f64,
    pub moisture: f64,
    /// Temperature `TempN(Node(i))` if not `lFlux`, or flux `vNew(Node(i))` when `lFlux`.
    pub temp_or_flux: f64,
    pub conc: Vec<f64>,
}

/// Full record across all observation nodes at time `t`.
#[derive(Clone, Debug, PartialEq)]
pub struct ObsNodeRow {
    pub time: f64,
    pub is_flux: bool,
    pub entries: Vec<ObsNodeEntry>,
}

/// Format `OBS_NODE.OUT` matching `INPUT.FOR:365-424` (headers) and `OUTPUT.FOR:931-1075`.
pub fn format_obs_node_out(
    node_indices: &[usize],
    num_species: usize,
    is_flux: bool,
    rows: &[ObsNodeRow],
) -> String {
    let mut out = String::new();
    let num_obs = node_indices.len().min(10);

    // Header line 1: Node labels
    let mut header1 = String::from("   Time   ");
    for &node_idx in node_indices.iter().take(num_obs) {
        let label = format!("Node({})", node_idx);
        let width = if num_species == 0 {
            29
        } else {
            29 + num_species * 12
        };
        header1.push_str(&format!("{:>width$}", label, width = width));
    }
    writeln!(out, "{}", header1).unwrap();

    // Header line 2: Field names
    let mut header2 = String::from("          ");
    let flux_label = if is_flux { "Flux" } else { "Temp" };
    for _ in 0..num_obs {
        header2.push_str(&format!("    h        theta    {:4}   ", flux_label));
        for _ in 0..num_species {
            header2.push_str("   Conc     ");
        }
    }
    writeln!(out, "{}", header2).unwrap();

    for r in rows {
        // Output format matching OUTPUT.FOR: 100-221
        let t_str = if r.time < 99999999.0 {
            format!("  {}", format_fortran_f(r.time, 14, 4))
        } else {
            format!(" {}", format_fortran_e(r.time, 15, 8))
        };
        let mut line = t_str;

        for entry in &r.entries {
            line.push_str(&format_fortran_f(entry.head, 12, 2));
            line.push_str(&format_fortran_f(entry.moisture, 8, 4));

            if entry.conc.is_empty() {
                if r.is_flux {
                    line.push_str(&format_fortran_e(entry.temp_or_flux, 11, 3));
                } else {
                    line.push_str(&format_fortran_f(entry.temp_or_flux, 9, 3));
                }
            } else if r.is_flux {
                line.push_str(&format_fortran_e(entry.temp_or_flux, 12, 4));
                for c in &entry.conc {
                    line.push_str(&format_fortran_e(*c, 12, 4));
                }
            } else {
                line.push_str(&format_fortran_f(entry.temp_or_flux, 9, 3));
                for c in &entry.conc {
                    line.push_str(&format_fortran_e(*c, 12, 4));
                }
            }
            line.push_str("  ");
        }
        writeln!(out, "{}", line).unwrap();
    }
    writeln!(out, "end").unwrap();
    out
}

/// Balance / sub-region output per sub-region layer.
#[derive(Clone, Debug, PartialEq)]
pub struct SubRegionLayer {
    pub area: f64,
    pub volume: f64,
    pub inflow: f64,
    pub h_mean: f64,
    pub sub_t: Option<f64>,
    pub t_mean: Option<f64>,
    pub conc_vol: Vec<f64>,
    pub conc_vol_im: Vec<f64>,
    pub sorb_vol_im: Vec<f64>,
    pub sorb_vol_im2: Vec<f64>,
    pub c_mean: Vec<f64>,
    pub c_mean_im: Vec<f64>,
}

/// Single species balance closures.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesBalance {
    pub species: usize,
    pub conc_bal_t: f64,
    pub conc_bal_r: Option<f64>,
}

/// Data block for `BALANCE.OUT` at time `t`.
#[derive(Clone, Debug, PartialEq)]
pub struct BalanceBlock {
    pub time: f64,
    pub is_initial: bool,
    pub total_area: f64,
    pub total_volume: f64,
    pub volume_im: Option<f64>,
    pub total_inflow: f64,
    pub total_h_mean: f64,
    pub total_heat_vol: Option<f64>,
    pub total_t_mean: Option<f64>,
    pub layers: Vec<SubRegionLayer>,
    pub top_flux: Option<f64>,
    pub bot_flux: Option<f64>,
    pub wat_bal_t: Option<f64>,
    pub wat_bal_r: Option<f64>,
    pub species_balances: Vec<SpeciesBalance>,
}

/// Format `BALANCE.OUT` matching `OUTPUT.FOR:325-716`.
pub fn format_balance_out(blocks: &[BalanceBlock]) -> String {
    let mut out = String::new();
    writeln!(out, " ******* Program HYDRUS").unwrap();
    writeln!(out).unwrap();

    for block in blocks {
        writeln!(
            out,
            "----------------------------------------------------------"
        )
        .unwrap();
        if block.time < 99999999.0 {
            writeln!(
                out,
                " Time       [T]{}",
                format_fortran_f(block.time, 14, 4)
            )
            .unwrap();
        } else {
            writeln!(
                out,
                " Time       [T]{}",
                format_fortran_e(block.time, 15, 8)
            )
            .unwrap();
        }
        writeln!(
            out,
            "----------------------------------------------------------"
        )
        .unwrap();

        // 120 format( ' Sub-region num.               ',9(I7,6x))
        let mut sub_reg_line = String::from(" Sub-region num.               ");
        for i in 1..=block.layers.len() {
            sub_reg_line.push_str(&format!("{:>7}      ", i));
        }
        writeln!(out, "{}", sub_reg_line).unwrap();
        writeln!(
            out,
            "----------------------------------------------------------"
        )
        .unwrap();

        // Area: 140 format( ' Area     [L]      ',e13.5,9e13.5)
        let mut area_line = format!(
            " Area     [L]      {}",
            format_fortran_e(block.total_area, 13, 5)
        );
        for lay in &block.layers {
            area_line.push_str(&format_fortran_e(lay.area, 13, 5));
        }
        writeln!(out, "{}", area_line).unwrap();

        // W-volume: 150 format( ' W-volume [L]      ',e13.5,9e13.5)
        let mut vol_line = format!(
            " W-volume [L]      {}",
            format_fortran_e(block.total_volume, 13, 5)
        );
        for lay in &block.layers {
            vol_line.push_str(&format_fortran_e(lay.volume, 13, 5));
        }
        writeln!(out, "{}", vol_line).unwrap();

        // Dual porosity immobile water volume: 151 format( ' W-volumeI[L]      ',e13.5,9e13.5)
        if let Some(vol_im) = block.volume_im {
            writeln!(
                out,
                " W-volumeI[L]      {}",
                format_fortran_e(vol_im, 13, 5)
            )
            .unwrap();
        }

        // In-flow: 160 format( ' In-flow  [L/T]    ',e13.5,9e13.5)
        let mut flow_line = format!(
            " In-flow  [L/T]    {}",
            format_fortran_e(block.total_inflow, 13, 5)
        );
        for lay in &block.layers {
            flow_line.push_str(&format_fortran_e(lay.inflow, 13, 5));
        }
        writeln!(out, "{}", flow_line).unwrap();

        // h Mean: 170 format( ' h Mean   [L]      ',e13.5,9e13.5)
        let mut h_line = format!(
            " h Mean   [L]      {}",
            format_fortran_e(block.total_h_mean, 13, 5)
        );
        for lay in &block.layers {
            h_line.push_str(&format_fortran_e(lay.h_mean, 13, 5));
        }
        writeln!(out, "{}", h_line).unwrap();

        // Heat:
        if let (Some(h_vol), Some(t_mean)) = (block.total_heat_vol, block.total_t_mean) {
            let mut hvol_line = format!(" HeatVol  [M/T2]   {}", format_fortran_e(h_vol, 13, 5));
            for lay in &block.layers {
                if let Some(st) = lay.sub_t {
                    hvol_line.push_str(&format_fortran_e(st, 13, 5));
                }
            }
            writeln!(out, "{}", hvol_line).unwrap();

            let mut tm_line = format!(" tMean    [K]      {}", format_fortran_f(t_mean, 13, 3));
            for lay in &block.layers {
                if let Some(tm) = lay.t_mean {
                    tm_line.push_str(&format_fortran_f(tm, 13, 3));
                }
            }
            writeln!(out, "{}", tm_line).unwrap();
        }

        // Solute species:
        let num_species = block.layers.first().map(|l| l.conc_vol.len()).unwrap_or(0);
        for js in 0..num_species {
            let sp_num = js + 1;
            // ConcVol
            let tot_conc_vol: f64 = block
                .layers
                .iter()
                .map(|l| l.conc_vol.get(js).copied().unwrap_or(0.0))
                .sum();
            let mut cv_line = format!(
                " ConcVol  [M/L2] {} {}",
                sp_num,
                format_fortran_e(tot_conc_vol, 13, 5)
            );
            for lay in &block.layers {
                cv_line.push_str(&format_fortran_e(
                    lay.conc_vol.get(js).copied().unwrap_or(0.0),
                    13,
                    5,
                ));
            }
            writeln!(out, "{}", cv_line).unwrap();

            // cMean
            let tot_c_mean = if block.total_area > 0.0 {
                block
                    .layers
                    .iter()
                    .map(|l| l.c_mean.get(js).copied().unwrap_or(0.0) * l.area)
                    .sum::<f64>()
                    / block.total_area
            } else {
                0.0
            };
            let mut cm_line = format!(
                " cMean    [M/L3] {} {}",
                sp_num,
                format_fortran_e(tot_c_mean, 13, 5)
            );
            for lay in &block.layers {
                cm_line.push_str(&format_fortran_e(
                    lay.c_mean.get(js).copied().unwrap_or(0.0),
                    13,
                    5,
                ));
            }
            writeln!(out, "{}", cm_line).unwrap();

            // Immobile solute if present
            let has_im = block.layers.iter().any(|l| !l.conc_vol_im.is_empty());
            if has_im {
                let tot_cv_im: f64 = block
                    .layers
                    .iter()
                    .map(|l| l.conc_vol_im.get(js).copied().unwrap_or(0.0))
                    .sum();
                let mut cv_im_line = format!(
                    " ConcVolIm[M/L2] {} {}",
                    sp_num,
                    format_fortran_e(tot_cv_im, 13, 5)
                );
                for lay in &block.layers {
                    cv_im_line.push_str(&format_fortran_e(
                        lay.conc_vol_im.get(js).copied().unwrap_or(0.0),
                        13,
                        5,
                    ));
                }
                writeln!(out, "{}", cv_im_line).unwrap();
            }

            let has_s_im2 = block.layers.iter().any(|l| !l.sorb_vol_im2.is_empty());
            if has_s_im2 {
                let tot_sv_im2: f64 = block
                    .layers
                    .iter()
                    .map(|l| l.sorb_vol_im2.get(js).copied().unwrap_or(0.0))
                    .sum();
                let mut sv_im2_line = format!(
                    " SorbVolIm2[M/L2]{} {}",
                    sp_num,
                    format_fortran_e(tot_sv_im2, 13, 5)
                );
                for lay in &block.layers {
                    sv_im2_line.push_str(&format_fortran_e(
                        lay.sorb_vol_im2.get(js).copied().unwrap_or(0.0),
                        13,
                        5,
                    ));
                }
                writeln!(out, "{}", sv_im2_line).unwrap();
            }
        }

        // Fluxes:
        if let (Some(top_f), Some(bot_f)) = (block.top_flux, block.bot_flux) {
            writeln!(out, " Top Flux [L/T]    {}", format_fortran_e(top_f, 13, 5)).unwrap();
            writeln!(out, " Bot Flux [L/T]    {}", format_fortran_e(bot_f, 13, 5)).unwrap();
        }

        // Balance closures:
        if !block.is_initial {
            if let Some(w_bal_t) = block.wat_bal_t {
                writeln!(
                    out,
                    " WatBalT  [L]      {}",
                    format_fortran_e(w_bal_t, 13, 5)
                )
                .unwrap();
            }
            if let Some(w_bal_r) = block.wat_bal_r {
                writeln!(
                    out,
                    " WatBalR  [%]      {}",
                    format_fortran_f(w_bal_r, 13, 3)
                )
                .unwrap();
            }
            for sp_bal in &block.species_balances {
                writeln!(
                    out,
                    " CncBalT  [M]    {} {}",
                    sp_bal.species,
                    format_fortran_e(sp_bal.conc_bal_t, 13, 5)
                )
                .unwrap();
                if let Some(c_bal_r) = sp_bal.conc_bal_r {
                    writeln!(
                        out,
                        " CncBalR  [%]    {} {}",
                        sp_bal.species,
                        format_fortran_f(c_bal_r, 13, 3)
                    )
                    .unwrap();
                }
            }
        }

        writeln!(
            out,
            "----------------------------------------------------------"
        )
        .unwrap();
        writeln!(out).unwrap();
    }
    writeln!(out, " Calculation time [sec]   0.0000000000000000     ").unwrap();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fortran_e_formatting() {
        assert_eq!(format_fortran_e(0.0, 13, 5), "  0.00000E+00");
        assert_eq!(format_fortran_e(0.12009, 13, 5), "  0.12009E+00");
        assert_eq!(format_fortran_e(-0.51638, 13, 5), " -0.51638E+00");
        assert_eq!(format_fortran_e(1.2009, 13, 5), "  0.12009E+01");
        assert_eq!(format_fortran_e(-0.43, 12, 4), " -0.4300E+00");
        assert_eq!(format_fortran_e(0.43, 12, 4), "  0.4300E+00");
    }

    #[test]
    fn format_fortran_e_rounds_exact_ties_to_even() {
        // Expected strings are gfortran 13.3.0 output for REAL values (DOUBLE for 2^-12).
        assert_eq!(format_fortran_e(49.0625, 13, 5), "  0.49062E+02");
        assert_eq!(format_fortran_e(-49.0625, 13, 5), " -0.49062E+02");
        assert_eq!(format_fortran_e(0.125, 11, 1), "    0.1E+00");
        assert_eq!(format_fortran_e(0.375, 11, 1), "    0.4E+00");
        assert_eq!(format_fortran_e(0.25, 11, 1), "    0.2E+00");
        assert_eq!(format_fortran_e(2.5, 11, 1), "    0.2E+01");
        assert_eq!(format_fortran_e(3.5, 11, 1), "    0.4E+01");
        // f32 9.99995 widened to f64: rounding carries into the exponent.
        assert_eq!(
            format_fortran_e(9.999_95_f32 as f64, 13, 5),
            "  0.10000E+02"
        );
        assert_eq!(format_fortran_e(2f64.powi(-12), 15, 8), " 0.24414062E-03");
    }

    #[test]
    fn test_fortran_f_formatting() {
        assert_eq!(format_fortran_f(0.1, 14, 4), "        0.1000");
        assert_eq!(format_fortran_f(-5.0, 10, 4), "   -5.0000");
        assert_eq!(format_fortran_f(20.0, 8, 2), "   20.00");
        assert_eq!(format_fortran_f(0.237, 13, 3), "        0.237");
    }

    #[test]
    fn test_profile_out_format() {
        let rows = vec![
            ProfileRow {
                n: 1,
                depth: 0.0,
                thr: 0.045,
                ths: 0.43,
                hs: -10.0,
                ks: 0.43,
                ks_kstop: 1.0,
                beta: 0.0,
                ah: 1.0,
                ak: 1.0,
                ath: 1.0,
            },
            ProfileRow {
                n: 2,
                depth: 5.0,
                thr: 0.045,
                ths: 0.43,
                hs: -10.0,
                ks: 0.43,
                ks_kstop: 1.0,
                beta: 0.0,
                ah: 1.0,
                ak: 1.0,
                ath: 1.0,
            },
        ];
        let text = format_profile_out(&rows);
        assert!(text.contains("Ks/KsTop"));
        assert!(text.ends_with("end\n"));
        let lines: Vec<&str> = text.lines().collect();
        // Skip header lines (line 0 is empty, line 1 is header, line 2 is empty, line 3 is row 1)
        let row1 = lines[3];
        let tokens: Vec<&str> = row1.split_whitespace().collect();
        assert_eq!(tokens.len(), 11);
        assert_eq!(tokens[0], "1");
        assert_eq!(tokens[1], "0.00");
    }

    #[test]
    fn test_nod_inf_out_format() {
        let blocks = vec![NodInfBlock {
            time: 0.1,
            num_species: 1,
            dual_porosity: false,
            rows: vec![
                NodInfRow {
                    node: 1,
                    depth: 0.0,
                    head: -10.0,
                    moisture: 0.3,
                    k: 0.1,
                    c: 0.01,
                    flux: 0.1,
                    sink: 0.0,
                    kappa: 1,
                    v_kstop: 1.0,
                    temp: 20.0,
                    conc: vec![0.15],
                    sorb: vec![0.025],
                    sorb2: vec![],
                },
                NodInfRow {
                    node: 2,
                    depth: -5.0,
                    head: -15.0,
                    moisture: 0.28,
                    k: 0.08,
                    c: 0.011,
                    flux: 0.1,
                    sink: 0.0,
                    kappa: 1,
                    v_kstop: 1.0,
                    temp: 20.0,
                    conc: vec![0.05],
                    sorb: vec![0.01],
                    sorb2: vec![],
                },
            ],
        }];
        let text = format_nod_inf_out(&blocks);
        assert!(text.contains("Time:        0.1000"));
        assert!(text.contains("Conc(1..NS)"));
        assert!(text.ends_with("end\n"));

        // Verify tokens per row
        let mut data_rows = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(first) = trimmed.split_whitespace().next() {
                if first == "1" || first == "2" {
                    data_rows.push(trimmed.split_whitespace().collect::<Vec<_>>());
                }
            }
        }
        assert_eq!(data_rows.len(), 2);
        assert_eq!(data_rows[0].len(), 13);
        assert_eq!(data_rows[0][0], "1");
        assert_eq!(data_rows[0][1], "0.0000");
        assert_eq!(data_rows[0][2], "-10.000");
    }

    #[test]
    fn test_solute_out_format_standard_and_flux() {
        let rows = vec![
            SoluteRow {
                time: 0.1,
                cv_top: 0.0,
                cv_bot: -0.51638,
                sum_cv_top: 0.0,
                sum_cv_bot: -0.051638,
                cv_ch0: 0.0,
                cv_ch1: 0.0,
                c_top: 0.81882,
                c_root: 0.0,
                c_bot: 1.2009,
                cv_root: 0.0,
                sum_cv_root: 0.0,
                sum_cv_ch1: 0.0,
                t_level: 1,
                c_gwl: 0.0,
                c_runoff: 0.0,
                sum_c_runoff: 0.0,
                obs_fluxes: vec![],
            },
            SoluteRow {
                time: 0.2,
                cv_top: 0.1,
                cv_bot: -0.5,
                sum_cv_top: 0.01,
                sum_cv_bot: -0.1,
                cv_ch0: 0.0,
                cv_ch1: 0.0,
                c_top: 0.6,
                c_root: 0.0,
                c_bot: 1.2,
                cv_root: 0.0,
                sum_cv_root: 0.0,
                sum_cv_ch1: 0.0,
                t_level: 2,
                c_gwl: 0.0,
                c_runoff: 0.0,
                sum_c_runoff: 0.0,
                obs_fluxes: vec![SoluteObsPair {
                    flux: 0.05,
                    cum_flux: 0.005,
                }],
            },
        ];
        let text = format_solute_out(&rows);
        assert!(text.contains("All solute fluxes"));
        assert!(text.ends_with("end\n"));
        let lines: Vec<&str> = text.lines().collect();
        // Row 1 (standard 17 columns)
        let row1_tokens: Vec<&str> = lines[4].split_whitespace().collect();
        assert_eq!(row1_tokens.len(), 17);
        assert_eq!(row1_tokens[0], "0.1000");

        // Row 2 (17 + 2 = 19 columns with 1 obs pair)
        let row2_tokens: Vec<&str> = lines[5].split_whitespace().collect();
        assert_eq!(row2_tokens.len(), 19);
    }

    #[test]
    fn test_obs_node_out_format() {
        let rows = vec![ObsNodeRow {
            time: 0.1,
            is_flux: false,
            entries: vec![
                ObsNodeEntry {
                    head: -10.0,
                    moisture: 0.3,
                    temp_or_flux: 20.0,
                    conc: vec![],
                },
                ObsNodeEntry {
                    head: -15.0,
                    moisture: 0.28,
                    temp_or_flux: 20.0,
                    conc: vec![],
                },
            ],
        }];
        let text = format_obs_node_out(&[1, 2], 0, false, &rows);
        assert!(text.contains("Node(1)"));
        assert!(text.contains("Node(2)"));
        assert!(text.ends_with("end\n"));

        let lines: Vec<&str> = text.lines().collect();
        let data_line = lines[2];
        let tokens: Vec<&str> = data_line.split_whitespace().collect();
        assert_eq!(tokens.len(), 7); // time, h(1), Th(1), Temp(1), h(2), Th(2), Temp(2)
        assert_eq!(tokens[0], "0.1000");
        assert_eq!(tokens[1], "-10.00");
    }

    #[test]
    fn test_balance_out_format() {
        let blocks = vec![BalanceBlock {
            time: 0.1,
            is_initial: false,
            total_area: 1.0,
            total_volume: 2.8,
            volume_im: Some(0.5),
            total_inflow: 0.0,
            total_h_mean: -12.5,
            total_heat_vol: None,
            total_t_mean: None,
            layers: vec![SubRegionLayer {
                area: 1.0,
                volume: 2.8,
                inflow: 0.0,
                h_mean: -12.5,
                sub_t: None,
                t_mean: None,
                conc_vol: vec![1.5],
                conc_vol_im: vec![],
                sorb_vol_im: vec![],
                sorb_vol_im2: vec![],
                c_mean: vec![0.8],
                c_mean_im: vec![],
            }],
            top_flux: Some(-0.43),
            bot_flux: Some(-0.43),
            wat_bal_t: Some(0.0),
            wat_bal_r: Some(0.0),
            species_balances: vec![SpeciesBalance {
                species: 1,
                conc_bal_t: -0.0012,
                conc_bal_r: Some(0.237),
            }],
        }];
        let text = format_balance_out(&blocks);
        assert!(text.contains("Time       [T]        0.1000"));
        assert!(text.contains("W-volume [L]"));
        assert!(text.contains("W-volumeI[L]"));
        assert!(text.contains("WatBalT  [L]"));
        assert!(text.contains("CncBalT  [M]"));
        assert!(text.contains("Calculation time"));
    }

    #[test]
    fn test_nod_inf_v_out_format() {
        let blocks = vec![NodInfVBlock {
            time: 0.1,
            rows: vec![NodInfVRow {
                node: 1,
                depth: 0.0,
                con: 0.1,
                con_lt: 0.0,
                con_vh: 0.01,
                con_vt: 0.002,
                v_liquid: 0.1,
                v_vapor: 0.012,
                v_total: 0.112,
                v_vap_iso: 0.01,
                v_vap_therm: 0.002,
            }],
        }];
        let text = format_nod_inf_v_out(&blocks);
        assert!(text.contains("Time:        0.1000"));
        assert!(text.contains("ConVh"));
        assert!(text.ends_with("end\n"));

        let mut data_rows = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(first) = trimmed.split_whitespace().next() {
                if first == "1" {
                    data_rows.push(trimmed.split_whitespace().collect::<Vec<_>>());
                }
            }
        }
        assert_eq!(data_rows.len(), 1);
        assert_eq!(data_rows[0].len(), 11);
        assert_eq!(data_rows[0][0], "1");
        assert_eq!(data_rows[0][1], "0.0000");
    }
}
