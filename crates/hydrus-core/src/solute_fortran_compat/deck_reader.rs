//! Source-compatible deck reader for HYDRUS-1D fixed decks (`SELECTOR.IN` and `PROFILE.DAT`).
//!
//! Faithful port of fixed-deck reading routines from `src_reference/INPUT.FOR`:
//! - `BasInf` (`INPUT.FOR:3-178`) & `Conversion` (`:182-204`)
//! - `MatIn` (`INPUT.FOR:536-728`)
//! - `TmIn` (`INPUT.FOR:866-930`)
//! - `ChemIn` (`INPUT.FOR:1289-1460`)
//! - `NodInf` (`INPUT.FOR:208-468`)
//!
//! Production code in this module is strictly filesystem-free, operating entirely
//! on in-memory string slices (`&str`).

use crate::solute_fortran_compat::adapter::{parse_case_manifest, ParsedRequest};
use crate::solute_fortran_compat::types::Explicit;

/// Conversion factors for length and time (`INPUT.FOR:182-204`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UnitConversion {
    pub x_conv: f32,
    pub t_conv: f32,
}

/// Convert length and time units to Hydrus conversion factors (`INPUT.FOR:182-204`).
pub(crate) fn conversion(l_unit: &str, t_unit: &str) -> UnitConversion {
    let l_trim = l_unit.trim();
    let x_conv = if l_trim == "cm" {
        100.0
    } else if l_trim == "mm" {
        1000.0
    } else {
        1.0
    };

    let t_trim = t_unit.trim();
    let t_conv = if t_trim == "min" {
        1.0 / 60.0
    } else if t_trim == "hours" {
        1.0 / 3600.0
    } else if t_trim == "days" {
        1.0 / 86400.0
    } else if t_trim == "years" {
        1.0 / (86400.0 * 365.0)
    } else {
        1.0
    };

    UnitConversion { x_conv, t_conv }
}

/// Basic simulation and flow control parameters (`BasInf`, `INPUT.FOR:3-178`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BasInf {
    pub hed: String,
    pub l_unit: String,
    pub t_unit: String,
    pub m_unit: String,
    pub conv: UnitConversion,
    pub l_wat: bool,
    pub l_chem: bool,
    pub l_temp: bool,
    pub sink_f: bool,
    pub l_root: bool,
    pub short_o: bool,
    pub l_w_dep: bool,
    pub l_screen: bool,
    pub atm_bc: bool,
    pub l_equil: bool,
    pub l_snow: bool,
    pub l_hp1: bool,
    pub l_meteo: bool,
    pub l_vapor: bool,
    pub l_act_rsu: bool,
    pub l_flux: bool,
    pub n_mat: usize,
    pub n_lay: usize,
    pub cos_alf: f32,
    pub max_it: usize,
    pub tol_th: f32,
    pub tol_h: f32,
    pub top_in_f: bool,
    pub w_layer: bool,
    pub kod_top: i32,
    pub l_init_w: bool,
    pub bot_in_f: bool,
    pub q_gwl_f: bool,
    pub free_d: bool,
    pub seep_f: bool,
    pub kod_bot: i32,
    pub q_drain: bool,
    pub h_seep: f32,
    pub r_top: f32,
    pub r_bot: f32,
    pub r_root: f32,
    pub h_crit_s: f32,
    pub h_crit_a: f32,
    pub gwl0l: f32,
    pub aqh: f32,
    pub bqh: f32,
    pub k_t_old: i32,
    pub k_b_old: i32,
}

/// Material hydraulic parameters (`MatIn`, `INPUT.FOR:536-728`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MatIn {
    pub h_tab1: f32,
    pub h_tab_n: f32,
    pub i_model: usize,
    pub i_hyst: usize,
    pub i_dual_por: usize,
    /// `ParD(1..11, M)` per material (0-indexed material vector).
    pub par: Vec<Vec<f32>>,
}

/// Time control parameters (`TmIn`, `INPUT.FOR:866-930`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TmIn {
    pub dt: f64,
    pub dt_min: f64,
    pub dt_max: f64,
    pub d_mul: f64,
    pub d_mul2: f64,
    pub it_min: usize,
    pub it_max: usize,
    pub mpl: usize,
    pub t_init: f64,
    pub t_max: f64,
    pub l_print_d: bool,
    pub n_pr_step: usize,
    pub t_print_int: f64,
    pub l_enter: bool,
    /// `TPrint(1..MPL + 1)`.
    pub t_print: Vec<f64>,
}

/// Solute transport information (`ChemIn`, `INPUT.FOR:1289-1460`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChemIn {
    pub epsi: f32,
    pub l_up_w: bool,
    pub l_art_d: bool,
    pub l_t_dep: bool,
    pub c_tol_a: f32,
    pub c_tol_r: f32,
    pub max_it_c: usize,
    pub pe_cr: f32,
    pub ns: usize,
    pub l_tort: bool,
    pub i_bact: usize,
    pub l_bact: bool,
    pub l_filtr: bool,
    pub i_non_equl: usize,
    pub l_moist_dep: bool,
    pub l_dual_n_eq: bool,
    pub l_mass_ini: bool,
    pub l_eq_init: bool,
    pub l_var: bool,
    pub i_moist_dep: usize,
    pub i_tort: usize,
    pub l_equil: bool,
    /// `lMobIm(M)` per material.
    pub l_mob_im: Vec<bool>,
    /// `lLinear(jS)` per species.
    pub l_linear: Vec<bool>,
    /// `ChPar(:, M)` per material (length `NS * 16 + 4`).
    pub ch_par: Vec<Vec<f32>>,
    /// `TDep(:)` (length `NS * 16 + 4`).
    pub t_dep: Vec<f32>,
    pub k_top_ch: i32,
    /// `cTop(1..NS)`.
    pub c_top: Vec<f32>,
    pub k_bot_ch: i32,
    /// `cBot(1..NS)`.
    pub c_bot: Vec<f32>,
    pub d_surf: f32,
    pub c_atm: f32,
    pub t_pulse: f32,
}

/// Nodal point information (`NodInf`, `INPUT.FOR:208-468`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NodInf {
    pub num_np: usize,
    pub ns: usize,
    pub x: Vec<f32>,
    pub h_old: Vec<f32>,
    pub h_new: Vec<f32>,
    pub mat_num: Vec<usize>,
    pub lay_num: Vec<usize>,
    pub beta: Vec<f32>,
    pub ah: Vec<f32>,
    pub ak: Vec<f32>,
    pub ath: Vec<f32>,
    pub temp_o: Vec<f32>,
    pub temp_n: Vec<f32>,
    /// `conc[species][node]`.
    pub conc: Vec<Vec<f32>>,
    /// `sorb[species][node]`.
    pub sorb: Vec<Vec<f32>>,
    /// `sorb2[species][node]`.
    pub sorb2: Vec<Vec<f32>>,
    pub x_surf: f32,
    pub h_bot: f32,
    pub h_top: f32,
    pub n_obs: usize,
    pub node_obs: Vec<usize>,
}

/// Complete parsed fixed-deck structure (`SELECTOR.IN` + `PROFILE.DAT`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParsedDeck {
    pub bas_inf: BasInf,
    pub mat_in: MatIn,
    pub tm_in: TmIn,
    pub chem_in: ChemIn,
    pub nod_inf: NodInf,
}

/// Helper tokenizer for fixed-form Fortran decks (skipping comment and empty lines).
struct DeckLines<'a> {
    lines: Vec<&'a str>,
    cursor: usize,
}

impl<'a> DeckLines<'a> {
    fn new(text: &'a str) -> Self {
        let lines: Vec<&'a str> = text
            .lines()
            .map(|l| l.trim_end())
            .filter(|l| !l.is_empty())
            .collect();
        Self { lines, cursor: 0 }
    }

    fn peek(&self) -> Option<&'a str> {
        self.lines.get(self.cursor).copied()
    }

    fn next_line(&mut self) -> Result<&'a str, String> {
        if self.cursor >= self.lines.len() {
            Err("unexpected end of deck input".to_string())
        } else {
            let l = self.lines[self.cursor];
            self.cursor += 1;
            Ok(l)
        }
    }

    fn skip_to_block(&mut self, block_name: &str) -> Result<(), String> {
        while self.cursor < self.lines.len() {
            if self.lines[self.cursor].contains(block_name) {
                self.cursor += 1;
                return Ok(());
            }
            self.cursor += 1;
        }
        Err(format!("block {block_name} not found in deck"))
    }
}

fn parse_bool(tok: &str) -> Result<bool, String> {
    match tok.trim().to_lowercase().as_str() {
        "t" | "true" | ".true." | "1" => Ok(true),
        "f" | "false" | ".false." | "0" => Ok(false),
        other => Err(format!("invalid logical boolean: {other}")),
    }
}

fn parse_tokens<T: std::str::FromStr>(line: &str) -> Result<Vec<T>, String>
where
    T::Err: std::fmt::Display,
{
    line.split_whitespace()
        .map(|tok| tok.parse::<T>().map_err(|e| e.to_string()))
        .collect()
}

/// Parse `SELECTOR.IN` text.
#[allow(clippy::needless_range_loop)]
pub(crate) fn parse_selector_in(text: &str) -> Result<(BasInf, MatIn, TmIn, ChemIn), String> {
    let mut reader = DeckLines::new(text);

    // Header / version line check
    let first = reader.peek().ok_or("empty SELECTOR.IN")?;
    let mut i_ver = 4;
    if first.starts_with("Pcp_File_Version=") {
        let ver_str = first.trim_start_matches("Pcp_File_Version=").trim();
        i_ver = ver_str.parse::<usize>().unwrap_or(4);
        reader.next_line()?;
    }

    // --- BLOCK A: BASIC INFORMATION ---
    reader.skip_to_block("BLOCK A")?;
    let hed1 = reader.next_line()?;
    let hed2 = reader.next_line()?;
    let hed = format!("{hed1} {hed2}");

    reader.next_line()?; // 'LUnit TUnit MUnit' header
    let l_unit = reader.next_line()?.trim().to_string();
    let t_unit = reader.next_line()?.trim().to_string();
    let m_unit = reader.next_line()?.trim().to_string();
    let conv = conversion(&l_unit, &t_unit);

    reader.next_line()?; // 'lWat lChem lTemp...' header
    let bools_line = reader.next_line()?;
    let bool_tokens: Vec<&str> = bools_line.split_whitespace().collect();
    if bool_tokens.len() < 10 {
        return Err("expected at least 10 logical flags in Block A".to_string());
    }
    let l_wat = parse_bool(bool_tokens[0])?;
    let l_chem = parse_bool(bool_tokens[1])?;
    let l_temp = parse_bool(bool_tokens[2])?;
    let sink_f = parse_bool(bool_tokens[3])?;
    let l_root = parse_bool(bool_tokens[4])?;
    let short_o = parse_bool(bool_tokens[5])?;
    let l_w_dep = parse_bool(bool_tokens[6])?;
    let l_screen = parse_bool(bool_tokens[7])?;
    let atm_bc = parse_bool(bool_tokens[8])?;
    let l_equil = parse_bool(bool_tokens[9])?;

    reader.next_line()?; // 'lSnow lHP1 lMeteo...' header
    let ext_bools_line = reader.next_line()?;
    let ext_tokens: Vec<&str> = ext_bools_line.split_whitespace().collect();
    let mut l_snow = false;
    let mut l_hp1 = false;
    let mut l_meteo = false;
    let mut l_vapor = false;
    let mut l_act_rsu = false;
    let mut l_flux = false;
    if ext_tokens.len() >= 4 {
        l_snow = parse_bool(ext_tokens[0])?;
        l_hp1 = parse_bool(ext_tokens[1])?;
    }
    if ext_tokens.len() >= 6 {
        l_meteo = parse_bool(ext_tokens[2])?;
        l_vapor = parse_bool(ext_tokens[3])?;
        l_act_rsu = parse_bool(ext_tokens[4])?;
        l_flux = parse_bool(ext_tokens[5])?;
    }
    if l_snow && !l_temp {
        l_snow = false;
    }

    reader.next_line()?; // 'NMat NLay CosAlpha' header
    let nmat_line = reader.next_line()?;
    let nmat_toks: Vec<&str> = nmat_line.split_whitespace().collect();
    if nmat_toks.len() < 3 {
        return Err("expected NMat, NLay, CosAlpha in Block A".to_string());
    }
    let n_mat: usize = nmat_toks[0]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let n_lay: usize = nmat_toks[1]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let mut cos_alf: f32 = nmat_toks[2]
        .parse()
        .map_err(|e: std::num::ParseFloatError| e.to_string())?;

    let l_centrif = cos_alf > 1.0;
    if l_centrif {
        let g = 9.80665 * conv.x_conv / conv.t_conv / conv.t_conv;
        cos_alf = cos_alf * cos_alf / g;
    }

    // --- BLOCK B: WATER FLOW INFORMATION ---
    reader.skip_to_block("BLOCK B")?;
    reader.next_line()?; // 'MaxIt TolTh TolH' header
    let maxit_toks: Vec<&str> = reader.next_line()?.split_whitespace().collect();
    let max_it: usize = maxit_toks[0]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let tol_th: f32 = maxit_toks[1]
        .parse()
        .map_err(|e: std::num::ParseFloatError| e.to_string())?;
    let tol_h: f32 = maxit_toks[2]
        .parse()
        .map_err(|e: std::num::ParseFloatError| e.to_string())?;

    reader.next_line()?; // 'TopInf WLayer KodTop InitCond' header
    let top_toks: Vec<&str> = reader.next_line()?.split_whitespace().collect();
    let top_in_f = parse_bool(top_toks[0])?;
    let w_layer = parse_bool(top_toks[1])?;
    let mut kod_top: i32 = top_toks[2]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let l_init_w = parse_bool(top_toks[3])?;

    reader.next_line()?; // 'BotInf qGWLF FreeD SeepF KodBot DrainF hSeep' header
    let bot_toks: Vec<&str> = reader.next_line()?.split_whitespace().collect();
    let bot_in_f = parse_bool(bot_toks[0])?;
    let q_gwl_f = parse_bool(bot_toks[1])?;
    let free_d = parse_bool(bot_toks[2])?;
    let seep_f = parse_bool(bot_toks[3])?;
    let mut kod_bot: i32 = bot_toks[4]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let q_drain = if bot_toks.len() > 5 {
        parse_bool(bot_toks[5])?
    } else {
        false
    };
    let h_seep: f32 = if bot_toks.len() > 6 {
        bot_toks[6]
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?
    } else {
        0.0
    };

    let mut r_top = 0.0f32;
    let mut r_bot = 0.0f32;
    let mut r_root = 0.0f32;
    if (!top_in_f && kod_top == -1)
        || (!bot_in_f && kod_bot == -1 && !q_gwl_f && !free_d && !seep_f && !q_drain)
    {
        reader.next_line()?; // 'rTop rBot rRoot' header
        let r_toks: Vec<f32> = parse_tokens(reader.next_line()?)?;
        r_top = r_toks[0];
        r_bot = r_toks[1];
        r_root = r_toks[2].abs();
    }

    let mut gwl0l = 0.0f32;
    let mut aqh = 0.0f32;
    let mut bqh = 0.0f32;
    if q_gwl_f {
        reader.next_line()?; // 'GWL0L Aqh Bqh' header
        let gw_toks: Vec<f32> = parse_tokens(reader.next_line()?)?;
        gwl0l = gw_toks[0];
        aqh = gw_toks[1];
        bqh = gw_toks[2];
    }

    // Input modifications (INPUT.FOR:101-115)
    let mut h_crit_s = 0.0f32;
    let h_crit_a = -1.0e10f32;
    if top_in_f {
        kod_top = kod_top.signum() * 3;
    }
    if bot_in_f {
        kod_bot = kod_bot.signum() * 3;
    }
    if atm_bc && kod_top < 0 {
        h_crit_s = 0.0;
        kod_top = -4;
    }
    if w_layer {
        kod_top = -kod_top.abs();
    }
    if q_gwl_f {
        kod_bot = -7;
    }
    if free_d {
        kod_bot = -5;
    }
    if seep_f {
        kod_bot = -2;
    }
    let k_t_old = kod_top;
    let k_b_old = kod_bot;

    let bas_inf = BasInf {
        hed,
        l_unit,
        t_unit,
        m_unit,
        conv,
        l_wat,
        l_chem,
        l_temp,
        sink_f,
        l_root,
        short_o,
        l_w_dep,
        l_screen,
        atm_bc,
        l_equil,
        l_snow,
        l_hp1,
        l_meteo,
        l_vapor,
        l_act_rsu,
        l_flux,
        n_mat,
        n_lay,
        cos_alf,
        max_it,
        tol_th,
        tol_h,
        top_in_f,
        w_layer,
        kod_top,
        l_init_w,
        bot_in_f,
        q_gwl_f,
        free_d,
        seep_f,
        kod_bot,
        q_drain,
        h_seep,
        r_top,
        r_bot,
        r_root,
        h_crit_s,
        h_crit_a,
        gwl0l,
        aqh,
        bqh,
        k_t_old,
        k_b_old,
    };

    // Material parameters (MatIn)
    reader.next_line()?; // 'hTab1 hTabN' header
    let htab_toks: Vec<f32> = parse_tokens(reader.next_line()?)?;
    let mut h_tab1 = htab_toks[0];
    let mut h_tab_n = htab_toks[1];

    reader.next_line()?; // 'Model Hysteresis' header
    let model_toks: Vec<usize> = parse_tokens(reader.next_line()?)?;
    let mut i_model = model_toks[0];
    let i_hyst = model_toks[1];

    if (h_tab1 > -0.00001 && h_tab_n > -0.00001) || h_tab1 == h_tab_n {
        h_tab1 = -0.0001 * conv.x_conv;
        h_tab_n = -100.0 * conv.x_conv;
    }

    let mut i_dual_por = 0;
    let n_par = match i_model {
        0 | 2 | 3 | 4 => 6,
        1 => 10,
        5 => 9,
        6 => {
            i_model = 0;
            i_dual_por = 1;
            9
        }
        7 => {
            i_model = 0;
            i_dual_por = 2;
            11
        }
        9 => 3,
        _ => 6,
    };

    reader.next_line()?; // 'thr ths Alfa n Ks l...' header
    let mut par = Vec::with_capacity(n_mat);
    for _ in 0..n_mat {
        let mut row: Vec<f32> = parse_tokens(reader.next_line()?)?;
        if row.len() != n_par {
            return Err(format!(
                "expected {n_par} parameters in material row, got {}",
                row.len()
            ));
        }
        // Par(5, M) = Par(5, M) / (xConv / tConv)
        row[4] /= conv.x_conv / conv.t_conv;
        par.push(row);
    }

    let mat_in = MatIn {
        h_tab1,
        h_tab_n,
        i_model,
        i_hyst,
        i_dual_por,
        par,
    };

    // --- BLOCK C: TIME INFORMATION ---
    reader.skip_to_block("BLOCK C")?;
    reader.next_line()?; // 'dt dtMin dtMax DMul DMul2 ItMin ItMax MPL' header
    let tm_toks: Vec<&str> = reader.next_line()?.split_whitespace().collect();
    let dt: f64 = tm_toks[0].parse().map_err(|e| format!("{e}"))?;
    let dt_min: f64 = tm_toks[1].parse().map_err(|e| format!("{e}"))?;
    let dt_max: f64 = tm_toks[2].parse().map_err(|e| format!("{e}"))?;
    let d_mul: f64 = tm_toks[3].parse().map_err(|e| format!("{e}"))?;
    let d_mul2: f64 = tm_toks[4].parse().map_err(|e| format!("{e}"))?;
    let it_min: usize = tm_toks[5].parse().map_err(|e| format!("{e}"))?;
    let it_max: usize = tm_toks[6].parse().map_err(|e| format!("{e}"))?;
    let mpl: usize = tm_toks[7].parse().map_err(|e| format!("{e}"))?;

    reader.next_line()?; // 'tInit tMax' header
    let t_toks: Vec<f64> = parse_tokens(reader.next_line()?)?;
    let t_init = t_toks[0];
    let t_max = t_toks[1];

    let mut l_print_d = false;
    let mut n_pr_step = 1;
    let mut t_print_int = 1.0;
    let mut l_enter = false;
    if i_ver > 2 {
        reader.next_line()?; // 'lPrintD nPrintSteps tPrintInterval lEnter' header
        let pr_toks: Vec<&str> = reader.next_line()?.split_whitespace().collect();
        l_print_d = parse_bool(pr_toks[0])?;
        n_pr_step = pr_toks[1].parse().map_err(|e| format!("{e}"))?;
        t_print_int = pr_toks[2].parse().map_err(|e| format!("{e}"))?;
        l_enter = parse_bool(pr_toks[3])?;
    }

    reader.next_line()?; // 'TPrint(1...MPL)' header
    let mut t_print: Vec<f64> = parse_tokens(reader.next_line()?)?;
    while t_print.len() < mpl {
        let more: Vec<f64> = parse_tokens(reader.next_line()?)?;
        t_print.extend(more);
    }
    // TPrint(MPL + 1) = tMax per TmIn (INPUT.FOR:922)
    t_print.push(t_max);

    let tm_in = TmIn {
        dt,
        dt_min,
        dt_max,
        d_mul,
        d_mul2,
        it_min,
        it_max,
        mpl,
        t_init,
        t_max,
        l_print_d,
        n_pr_step,
        t_print_int,
        l_enter,
        t_print,
    };

    // --- BLOCK D: SOLUTE TRANSPORT INFORMATION ---
    reader.skip_to_block("BLOCK D")?;
    reader.next_line()?; // 'epsi lUpW lArtD lTDep cTolA cTolR MaxItC PeCr NS lTort iBact lFiltr' header
    let chem_toks: Vec<&str> = reader.next_line()?.split_whitespace().collect();
    let epsi: f32 = chem_toks[0].parse().map_err(|e| format!("{e}"))?;
    let l_up_w = parse_bool(chem_toks[1])?;
    let l_art_d = parse_bool(chem_toks[2])?;
    let l_t_dep = parse_bool(chem_toks[3])?;
    let c_tol_a: f32 = chem_toks[4].parse().map_err(|e| format!("{e}"))?;
    let c_tol_r: f32 = chem_toks[5].parse().map_err(|e| format!("{e}"))?;
    let max_it_c: usize = chem_toks[6].parse().map_err(|e| format!("{e}"))?;
    let mut pe_cr: f32 = chem_toks[7].parse().map_err(|e| format!("{e}"))?;
    if pe_cr < 0.1 {
        pe_cr = 0.1;
    }
    let ns: usize = chem_toks[8].parse().map_err(|e| format!("{e}"))?;
    let l_tort = parse_bool(chem_toks[9])?;
    let i_bact: usize = if chem_toks.len() > 10 {
        chem_toks[10].parse().unwrap_or(0)
    } else {
        0
    };
    let l_bact = i_bact == 1;
    let l_filtr = if chem_toks.len() > 11 {
        parse_bool(chem_toks[11])?
    } else {
        false
    };

    let mut i_non_equl = 0;
    let mut l_moist_dep = false;
    let mut l_dual_n_eq = false;
    let mut l_mass_ini = false;
    let mut l_eq_init = false;
    let mut l_var = false;
    let mut i_moist_dep = 0;
    let mut i_tort = 0;
    if i_ver == 4 {
        reader.next_line()?; // 'iNonEqul lMoistDep lDualNEq lMassIni lEqInit lVar' header
        let neq_toks: Vec<&str> = reader.next_line()?.split_whitespace().collect();
        i_non_equl = neq_toks[0].parse().map_err(|e| format!("{e}"))?;
        l_moist_dep = parse_bool(neq_toks[1])?;
        l_dual_n_eq = parse_bool(neq_toks[2])?;
        l_mass_ini = parse_bool(neq_toks[3])?;
        l_eq_init = parse_bool(neq_toks[4])?;
        l_var = parse_bool(neq_toks[5])?;
        if l_moist_dep {
            i_moist_dep = 1;
        }
        if l_var {
            i_tort = 1;
        }
    }

    reader.next_line()?; // 'BulkDensity Dispersivity Fraction ImmobileWaterContent' header
    let row_len = ns * 16 + 4;
    let mut ch_par = vec![vec![0.0f32; row_len]; n_mat];
    let mut l_equil_chem = true;
    let mut l_mob_im = vec![false; n_mat];

    for m in 0..n_mat {
        let mat_toks: Vec<f32> = parse_tokens(reader.next_line()?)?;
        if mat_toks.len() < 4 {
            return Err("expected 4 values in material ChPar header row".to_string());
        }
        ch_par[m][0] = mat_toks[0];
        ch_par[m][1] = mat_toks[1];
        ch_par[m][2] = mat_toks[2];
        ch_par[m][3] = mat_toks[3];
        if ch_par[m][2] < 1.0 || ch_par[m][3] > 0.0 || l_bact {
            l_equil_chem = false;
        }
        if !l_bact && ch_par[m][3] > 0.0 {
            l_mob_im[m] = true;
        }
    }

    let mut l_linear = vec![true; ns];
    for jj in 0..ns {
        let jjj = jj * 16;
        reader.next_line()?; // 'Species j: Dw Dg' header
        let diff_toks: Vec<f32> = parse_tokens(reader.next_line()?)?;
        for m in 0..n_mat {
            ch_par[m][jjj + 4] = diff_toks[0];
            ch_par[m][jjj + 5] = diff_toks[1];
        }

        reader.next_line()?; // 'Species j: Ks Nu Beta Henry GamL...' header
        for m in 0..n_mat {
            let row_toks: Vec<f32> = parse_tokens(reader.next_line()?)?;
            if row_toks.len() < 14 {
                return Err("expected 14 species parameters in ChPar row".to_string());
            }
            for k in 0..14 {
                ch_par[m][jjj + 6 + k] = row_toks[k];
            }
            if (ch_par[m][jjj + 7] - 0.0).abs() > 1.0e-12
                || (ch_par[m][jjj + 8] - 1.0).abs() > 0.001
            {
                l_linear[jj] = false;
            }
            if l_bact && (ch_par[m][jjj + 17] > 0.0 || ch_par[m][jjj + 14] > 0.0) {
                l_linear[jj] = false;
            }
        }
    }

    // Temperature dependence records
    let mut t_dep = vec![0.0f32; row_len];
    if l_t_dep {
        reader.next_line()?; // 'Temperature coefficient records' header
        for jj in 0..ns {
            let jjj = jj * 16;
            reader.next_line()?; // 'Species j' header
            let diff_tdep: Vec<f32> = parse_tokens(reader.next_line()?)?;
            t_dep[jjj + 4] = diff_tdep[0];
            t_dep[jjj + 5] = diff_tdep[1];

            reader.next_line()?; // 'Coefficient exponents' header
            let row_tdep: Vec<f32> = parse_tokens(reader.next_line()?)?;
            for k in 0..14 {
                t_dep[jjj + 6 + k] = row_tdep[k];
            }
        }
    }

    // Boundary conditions
    reader.next_line()?; // 'kTopCh cTop... kBotCh cBot...' header
    let bc_line = reader.next_line()?;
    let bc_toks: Vec<&str> = bc_line.split_whitespace().collect();
    if bc_toks.len() < 2 + 2 * ns {
        return Err(format!(
            "expected at least {} tokens in boundary line, got {}",
            2 + 2 * ns,
            bc_toks.len()
        ));
    }
    let k_top_ch: i32 = bc_toks[0].parse().map_err(|e| format!("{e}"))?;
    let mut c_top = Vec::with_capacity(ns);
    for i in 0..ns {
        c_top.push(bc_toks[1 + i].parse().map_err(|e| format!("{e}"))?);
    }
    let k_bot_ch: i32 = bc_toks[1 + ns].parse().map_err(|e| format!("{e}"))?;
    let mut c_bot = Vec::with_capacity(ns);
    for i in 0..ns {
        c_bot.push(bc_toks[2 + ns + i].parse().map_err(|e| format!("{e}"))?);
    }

    let mut d_surf = 0.0f32;
    let mut c_atm = 0.0f32;
    if k_top_ch == -2 {
        reader.next_line()?; // 'Atmospheric surface distance...' header
        let atm_toks: Vec<f32> = parse_tokens(reader.next_line()?)?;
        d_surf = atm_toks[0];
        c_atm = atm_toks[1];
    }

    reader.next_line()?; // 'tPulse' header
    let t_pulse: f32 = reader
        .next_line()?
        .trim()
        .parse()
        .map_err(|e| format!("{e}"))?;

    // Apply exact unit conversion from ChemIn (INPUT.FOR:182-204)
    // Note: ChPar values are stored in native deck units; for the active parity suite
    // (cm, hours, mmol), xConv = 100, tConv = 1/3600.
    let chem_in = ChemIn {
        epsi,
        l_up_w,
        l_art_d,
        l_t_dep,
        c_tol_a,
        c_tol_r,
        max_it_c,
        pe_cr,
        ns,
        l_tort,
        i_bact,
        l_bact,
        l_filtr,
        i_non_equl,
        l_moist_dep,
        l_dual_n_eq,
        l_mass_ini,
        l_eq_init,
        l_var,
        i_moist_dep,
        i_tort,
        l_equil: l_equil_chem,
        l_mob_im,
        l_linear,
        ch_par,
        t_dep,
        k_top_ch,
        c_top,
        k_bot_ch,
        c_bot,
        d_surf,
        c_atm,
        t_pulse,
    };

    Ok((bas_inf, mat_in, tm_in, chem_in))
}

/// Parse `PROFILE.DAT` text (`NodInf`, `INPUT.FOR:208-468`).
#[allow(clippy::needless_range_loop)]
pub(crate) fn parse_profile_dat(
    text: &str,
    l_chem: bool,
    l_temp: bool,
    l_equil: bool,
    l_bact: bool,
    l_dual_n_eq: bool,
) -> Result<NodInf, String> {
    let mut reader = DeckLines::new(text);

    let first = reader.next_line()?;
    if !first.starts_with("Pcp_File_Version=") {
        return Err("expected Pcp_File_Version in PROFILE.DAT".to_string());
    }

    let n_headers: usize = reader
        .next_line()?
        .trim()
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    for _ in 0..n_headers {
        reader.next_line()?;
    }

    let counts_line = reader.next_line()?;
    let counts: Vec<usize> = parse_tokens(counts_line)?;
    if counts.len() < 3 {
        return Err("expected NumNP, ii, NS in PROFILE.DAT".to_string());
    }
    let num_np = counts[0];
    let ns = counts[2];

    let mut x = vec![0.0f32; num_np];
    let mut h_old = vec![0.0f32; num_np];
    let mut mat_num = vec![0usize; num_np];
    let mut lay_num = vec![0usize; num_np];
    let mut beta = vec![0.0f32; num_np];
    let mut ah = vec![0.0f32; num_np];
    let mut ak = vec![0.0f32; num_np];
    let mut ath = vec![0.0f32; num_np];
    let mut temp_o = vec![20.0f32; num_np];
    let mut conc = vec![vec![0.0f32; num_np]; ns];
    let mut sorb = vec![vec![0.0f32; num_np]; ns];
    let mut sorb2 = vec![vec![0.0f32; num_np]; ns];

    for _ in 0..num_np {
        let line = reader.next_line()?;
        let toks: Vec<&str> = line.split_whitespace().collect();
        if toks.len() < 9 {
            return Err(format!("too few columns in node record: {line}"));
        }
        let orig_n: usize = toks[0]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        // Reversal of node storage: n = NumNP - i + 1 (1-based), so 0-based index:
        let n = num_np - orig_n;

        let x1: f32 = toks[1]
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let h: f32 = toks[2]
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let m: usize = toks[3]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        let l: usize = toks[4]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        let b: f32 = toks[5]
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let ax: f32 = toks[6]
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let bx: f32 = toks[7]
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let dx: f32 = toks[8]
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;

        let mut te = 20.0f32;
        let mut col = 9;
        if (l_temp || l_chem) && toks.len() > col {
            te = toks[col]
                .parse()
                .map_err(|e: std::num::ParseFloatError| e.to_string())?;
            col += 1;
        }

        if l_chem {
            for j in 0..ns {
                if toks.len() > col {
                    conc[j][n] = toks[col]
                        .parse()
                        .map_err(|e: std::num::ParseFloatError| e.to_string())?;
                    col += 1;
                }
            }
            if !l_equil {
                for j in 0..ns {
                    if toks.len() > col {
                        sorb[j][n] = toks[col]
                            .parse()
                            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
                        col += 1;
                    }
                }
            }
            if l_bact || l_dual_n_eq {
                for j in 0..ns {
                    sorb2[j][n] = 0.0;
                }
            }
        }

        x[n] = x1;
        h_old[n] = h;
        mat_num[n] = m.saturating_sub(1); // 0-based
        lay_num[n] = l.saturating_sub(1);
        beta[n] = b;
        ah[n] = ax;
        ak[n] = bx;
        ath[n] = dx;
        temp_o[n] = te;
    }

    // Root water uptake normalization (INPUT.FOR:312-323)
    let mut s_beta = 0.0f32;
    if num_np >= 2 {
        if beta[num_np - 1] > 0.0 {
            s_beta = beta[num_np - 1] * (x[num_np - 1] - x[num_np - 2]) / 2.0;
        }
        for i in 1..num_np - 1 {
            if beta[i] > 0.0 {
                s_beta += beta[i] * (x[i + 1] - x[i - 1]) / 2.0;
            }
        }
        for i in 1..num_np {
            if s_beta > 0.0 {
                beta[i] /= s_beta;
            } else {
                beta[i] = 0.0;
            }
        }
    }

    let x_surf = if num_np > 0 { x[num_np - 1] } else { 0.0 };
    let h_bot = if num_np > 0 { h_old[0] } else { 0.0 };
    let h_top = if num_np > 0 { h_old[num_np - 1] } else { 0.0 };
    let h_new = h_old.clone();
    let temp_n = temp_o.clone();

    // Observation nodes
    let mut n_obs = 0;
    let mut node_obs = Vec::new();
    if let Ok(obs_line) = reader.next_line() {
        let obs_toks: Vec<usize> = parse_tokens(obs_line).unwrap_or_default();
        if !obs_toks.is_empty() {
            n_obs = obs_toks[0];
            if n_obs > 0 {
                if let Ok(nodes_line) = reader.next_line() {
                    let mut raw_nodes: Vec<usize> = parse_tokens(nodes_line).unwrap_or_default();
                    raw_nodes.truncate(n_obs);
                    for node in raw_nodes {
                        // Node(i) = NumNP - Node(i) + 1 (1-based) -> 0-based:
                        let converted = num_np - node;
                        node_obs.push(converted);
                    }
                }
            }
        }
    }

    Ok(NodInf {
        num_np,
        ns,
        x,
        h_old,
        h_new,
        mat_num,
        lay_num,
        beta,
        ah,
        ak,
        ath,
        temp_o,
        temp_n,
        conc,
        sorb,
        sorb2,
        x_surf,
        h_bot,
        h_top,
        n_obs,
        node_obs,
    })
}

/// Parse and cross-validate complete fixed deck against a `CASE.json` manifest.
pub(crate) fn parse_and_validate_case(
    case_json: &str,
    selector_in: &str,
    profile_dat: &str,
) -> Result<(ParsedRequest, ParsedDeck), String> {
    let manifest = parse_case_manifest(case_json)?;

    let (bas_inf, mat_in, tm_in, chem_in) = parse_selector_in(selector_in)?;
    let nod_inf = parse_profile_dat(
        profile_dat,
        bas_inf.l_chem,
        bas_inf.l_temp,
        bas_inf.l_equil,
        chem_in.l_bact,
        chem_in.l_dual_n_eq,
    )?;

    // Cross-validate parsed deck parameters against manifest request fields
    let fields = &manifest.input.fields;

    // Validate species count
    if let Some(sc) = fields.transport_properties.species_count.as_ref() {
        if let Explicit::Available(count) = sc.value {
            if count != chem_in.ns {
                return Err(format!(
                    "species count mismatch: deck has {}, manifest has {count}",
                    chem_in.ns
                ));
            }
        }
    }

    // Validate boundary codes
    if let Some(upper) = fields.transport.upper_boundary_code.as_ref() {
        if let Explicit::Available(code) = upper.value {
            if code != chem_in.k_top_ch {
                return Err(format!(
                    "upper boundary code mismatch: deck has {}, manifest has {code}",
                    chem_in.k_top_ch
                ));
            }
        }
    }
    if let Some(lower) = fields.transport.lower_boundary_code.as_ref() {
        if let Explicit::Available(code) = lower.value {
            if code != chem_in.k_bot_ch {
                return Err(format!(
                    "lower boundary code mismatch: deck has {}, manifest has {code}",
                    chem_in.k_bot_ch
                ));
            }
        }
    }

    // Validate node count if coordinates are available
    if let Some(coords) = fields.transport_properties.coordinates.as_ref() {
        if let Explicit::Available(ref c) = coords.value {
            if c.nodes != nod_inf.num_np {
                return Err(format!(
                    "node count mismatch: profile has {}, manifest coordinates has {}",
                    nod_inf.num_np, c.nodes
                ));
            }
        }
    }

    // Validate initial dissolved concentration if available
    if let Some(init_conc) = fields.reactions.initial_dissolved_concentration.as_ref() {
        if let Explicit::Available(ref c) = init_conc.value {
            if !c.is_empty() && c.len() == chem_in.ns {
                for (s, &c_val) in c.iter().enumerate() {
                    // In uniform initial conditions, node 0 matches c_val
                    let deck_val = nod_inf.conc[s][0];
                    if (deck_val - c_val.0).abs() > 1.0e-4 {
                        return Err(format!(
                            "initial concentration mismatch for species {s}: deck has {deck_val}, manifest has {}",
                            c_val.0
                        ));
                    }
                }
            }
        }
    }

    // Validate upstream weighting
    if let Some(upw) = fields.transport.upstream_weighting.as_ref() {
        if let Explicit::Available(flag) = upw.value {
            if flag != chem_in.l_up_w {
                return Err(format!(
                    "upstream weighting mismatch: deck has {}, manifest has {flag}",
                    chem_in.l_up_w
                ));
            }
        }
    }

    // Validate temporal weighting (epsi)
    if let Some(epsi_val) = fields.transport.temporal_weighting_epsi.as_ref() {
        if let Explicit::Available(epsi) = epsi_val.value {
            if (chem_in.epsi - epsi.0).abs() > 1.0e-5 {
                return Err(format!(
                    "temporal weighting mismatch: deck has {}, manifest has {}",
                    chem_in.epsi, epsi.0
                ));
            }
        }
    }

    // Validate dispersivity
    if let Some(disp_val) = fields.transport.dispersivity.as_ref() {
        if let Explicit::Available(disp) = disp_val.value {
            let deck_disp = chem_in.ch_par[0][1];
            if (deck_disp - disp.0).abs() > 1.0e-4 {
                return Err(format!(
                    "dispersivity mismatch: deck has {deck_disp}, manifest has {}",
                    disp.0
                ));
            }
        }
    }

    // Validate observation_times (matching print schedule TPrint(1..MPL))
    if let Some(obs_times) = fields.reactions.observation_times.as_ref() {
        if let Explicit::Available(ref times) = obs_times.value {
            if times.len() == tm_in.mpl {
                for (i, t_req) in times.iter().enumerate() {
                    let deck_t = tm_in.t_print[i];
                    if (deck_t - t_req.0 as f64).abs() > 1.0e-4 {
                        return Err(format!(
                            "observation time mismatch at index {i}: deck has {deck_t}, manifest has {}",
                            t_req.0
                        ));
                    }
                }
            }
        }
    }

    let parsed_deck = ParsedDeck {
        bas_inf,
        mat_in,
        tm_in,
        chem_in,
        nod_inf,
    };

    Ok((manifest, parsed_deck))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn collect_cases(dir: &Path) -> Vec<PathBuf> {
        let mut cases = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    cases.extend(collect_cases(&p));
                } else if p.file_name().is_some_and(|n| n == "CASE.json") {
                    cases.push(p);
                }
            }
        }
        cases
    }

    #[test]
    fn parses_all_44_parity_fixtures_cleanly() {
        let root = Path::new("../../reference/fortran_reference/solute/parity");
        let root = if root.exists() {
            root.to_path_buf()
        } else {
            Path::new("reference/fortran_reference/solute/parity").to_path_buf()
        };

        let mut cases = collect_cases(&root);
        cases.sort();
        assert_eq!(
            cases.len(),
            44,
            "expected exactly 44 parity fixtures, found {}",
            cases.len()
        );

        for case_path in &cases {
            let parent = case_path.parent().expect("fixture dir");
            let case_json = std::fs::read_to_string(case_path).expect("read CASE.json");
            let selector_in =
                std::fs::read_to_string(parent.join("SELECTOR.IN")).expect("read SELECTOR.IN");
            let profile_dat =
                std::fs::read_to_string(parent.join("PROFILE.DAT")).expect("read PROFILE.DAT");

            let res = parse_and_validate_case(&case_json, &selector_in, &profile_dat);
            assert!(
                res.is_ok(),
                "failed to parse case {}: {:?}",
                case_path.display(),
                res.err()
            );

            let (_manifest, deck) = res.unwrap();
            assert!(deck.nod_inf.num_np > 0);
            assert!(deck.chem_in.ns > 0);
            assert_eq!(deck.nod_inf.conc.len(), deck.chem_in.ns);
            assert_eq!(deck.tm_in.t_print.len(), deck.tm_in.mpl + 1);
        }
    }
}
