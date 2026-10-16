//! Source-mapped compatibility data contracts. Values retain source precision explicitly.

/// Error type shared by compatibility kernels; static messages, not owned strings.
pub(crate) type KernelError = &'static str;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DefaultReal(pub f32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DoubleReal(pub f64);

impl DefaultReal {
    pub(crate) fn from_input(value: f64) -> Self {
        Self(value as f32)
    }
    pub(crate) fn to_double(self) -> DoubleReal {
        DoubleReal(self.0 as f64)
    }
}

impl DoubleReal {
    /// Mirrors `sngl(F(i))` followed by the active nonnegative concentration clamp.
    pub(crate) fn accepted_concentration(self) -> DefaultReal {
        DefaultReal((self.0 as f32).max(0.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoundaryForm {
    Prescribed,
    TypeThree,
    ZeroGradient,
    Atmospheric,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlowOrientation {
    PositiveNative,
    NegativeNative,
}

/// Attachment blocking function selection (`iPsi`).
///
/// `Type3` corresponds to `iPsi=3` (`src_reference/SOLUTE.FOR:1570-1579`),
/// which is active via column-6 `!` continuation lines (`SOLUTE.FOR:1576, 1578`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockingMode {
    Default,
    Type1,
    Type2,
    Type3,
    Type4,
    Type5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SorptionMode {
    Equilibrium,
    MobileImmobile,
    DualDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceSwitches {
    pub upstream: bool,
    pub artificial_dispersion: bool,
    pub tortuosity: bool,
    pub temperature: bool,
    pub vapor: bool,
    pub gas_terms: bool,
    pub equilibrium: bool,
    pub linear: bool,
    pub mobile_immobile: bool,
    pub dual_domain_model: Option<u8>,
    pub bacteria: bool,
    pub filtration: bool,
    pub blocking: BlockingMode,
    pub flux_concentration: bool,
    pub moisture_selector: u8,
    pub mass_initialization: bool,
    pub equilibrium_initialization: bool,
    /// `SinkF`, the root-uptake selector of the `Site.dat` control record read at
    /// `INPUT.FOR:31` (`read(30,*,err=901) lWat,lChem,lTemp,SinkF,lRoot,ShortO,
    /// lWDep,` with `lScreen,AtmBC,lEquil` on the column-6 continuation `32`).
    /// `SinkF` is assigned nowhere else in `src_reference`, so this is its single
    /// home: it is the `SOLUTE.FOR:94` `if(SinkF)` call guard that the `SetSSnk`
    /// continuation of `95-96` hangs from.
    pub root_sink: bool,
    /// `lWat`, the same control record's water-flow selector (`INPUT.FOR:31`) and
    /// likewise assigned nowhere else. This is the single home of the second term
    /// of the `HYDRUS.FOR:390` guard `if(lBact.and..not.lWat)`, so the `Exclusion`
    /// call of the column-6 continuation `391` can only run when this is false.
    pub water_flow: bool,
}

impl Default for SourceSwitches {
    fn default() -> Self {
        Self {
            upstream: false,
            artificial_dispersion: false,
            tortuosity: false,
            temperature: false,
            vapor: false,
            gas_terms: false,
            equilibrium: true,
            linear: true,
            mobile_immobile: false,
            dual_domain_model: None,
            bacteria: false,
            filtration: false,
            blocking: BlockingMode::Default,
            flux_concentration: false,
            moisture_selector: 0,
            mass_initialization: false,
            equilibrium_initialization: false,
            root_sink: false,
            water_flow: false,
        }
    }
}

impl SourceSwitches {
    pub(crate) fn validate(self, species_count: usize) -> Result<(), KernelError> {
        if self.bacteria && species_count > 1 {
            return Err("bacteria with a multi-species chain is unsupported");
        }
        if let Some(model) = self.dual_domain_model {
            if !matches!(model, 6 | 7) {
                return Err("dual-domain water model must be 6 or 7");
            }
            if self.mobile_immobile {
                return Err("mobile-immobile and dual-domain selectors conflict");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SourceIndices {
    /// For each canonical node, its one-based native Fortran node index.
    pub canonical_to_native: Vec<usize>,
}

impl SourceIndices {
    pub(crate) fn identity(nodes: usize) -> Self {
        Self {
            canonical_to_native: (1..=nodes).collect(),
        }
    }
    pub(crate) fn validate(&self, nodes: usize) -> Result<(), KernelError> {
        if self.canonical_to_native.len() != nodes {
            return Err("node permutation length mismatch");
        }
        let mut seen = vec![false; nodes];
        for &index in &self.canonical_to_native {
            if index == 0 || index > nodes || seen[index - 1] {
                return Err("node permutation must be a one-based bijection");
            }
            seen[index - 1] = true;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct UnavailableValue {
    pub availability: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Explicit<T> {
    Available(T),
    Unavailable(UnavailableValue),
}

impl<T> Explicit<T> {
    pub(crate) fn unavailable_reason(&self) -> Option<&str> {
        match self {
            Self::Available(_) => None,
            Self::Unavailable(u) => Some(&u.reason),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct UnitValue<T> {
    pub units: String,
    pub value: Explicit<T>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CoordinatesSpec {
    pub native: String,
    pub canonical: String,
    pub nodes: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TransportInputs {
    pub temporal_weighting_epsi: Option<UnitValue<DefaultReal>>,
    pub time_interval: Option<UnitValue<DefaultReal>>,
    pub dispersivity: Option<UnitValue<DefaultReal>>,
    pub molecular_diffusion: Option<UnitValue<DefaultReal>>,
    pub upstream_weighting: Option<UnitValue<bool>>,
    pub upper_boundary_code: Option<UnitValue<i32>>,
    pub lower_boundary_code: Option<UnitValue<i32>>,
    pub flow_orientation: Option<UnitValue<FlowOrientation>>,
    pub atmospheric_concentration: Option<UnitValue<DefaultReal>>,
    pub atmospheric_surface_distance: Option<UnitValue<DefaultReal>>,
    pub flux_concentration: Option<UnitValue<bool>>,
    pub vapor_lower_boundary_exception: Option<UnitValue<bool>>,
    pub endpoint_state: Option<UnitValue<EndpointPrimitiveState>>,
}

/// Primitives of one species time step, mirroring the arguments that
/// `MatSet` (`SOLUTE.FOR:755-759`) and the `Solute` endpoint flux closure
/// (`65-84`, `197-225`) receive. No preassembled row, right-hand side,
/// residual, endpoint flux or atmospheric coefficient is representable here.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EndpointPrimitiveState {
    /// `x` (`763`), in native node order.
    pub coordinates: Vec<DefaultReal>,
    /// `thO` (`763`) before the `MatSet` mobile/immobile adjustment (`768-776`).
    pub old_water: Vec<DefaultReal>,
    /// `thN` (`763`) before the same adjustment.
    pub new_water: Vec<DefaultReal>,
    /// `vO` and `vN` as `Solute` holds them where the flux closure runs: `vO`
    /// before the level loop (phase A, `65-72`) and `vN` after this step's
    /// `vCorr` restoration (`129-132`, phase B, `197-221`).
    pub old_velocity: Vec<DefaultReal>,
    pub new_velocity: Vec<DefaultReal>,
    /// `vO` and `vN` as both `MatSet` calls receive them (`124-128`), i.e.
    /// before `vCorr` is added at `129-132`.
    pub old_matrix_velocity: Vec<DefaultReal>,
    pub new_matrix_velocity: Vec<DefaultReal>,
    /// `Disp` at `Level=1`/`Level=2` (`763`, `112-118`).
    pub old_dispersion: Vec<DefaultReal>,
    pub new_dispersion: Vec<DefaultReal>,
    /// `Retard` at `Level=1`/`Level=2` (`764`, `112-118`).
    pub old_retardation: Vec<DefaultReal>,
    pub new_retardation: Vec<DefaultReal>,
    /// `Conc(jS,·)` while the matrix is assembled and at flux phase A (`85`).
    pub old_concentration: Vec<DefaultReal>,
    /// `Conc(jS,·)` after `BanSol` and the `148-179` update, which flux phase B
    /// reads (`204`, `211`, `216`, `221`).
    pub new_accepted_concentration: Vec<DefaultReal>,
    /// `ChPar(4,M)`, subtracted from both waters by `MatSet` (`771`, `932`).
    pub material_immobile_water: Vec<DefaultReal>,
    /// `g0` and `g1` as the `Level=1` `Coeff` call left them (`112-118`); the
    /// `Level=1` `MatSet` call reads these (`781-789`, `869-877`).
    pub old_source_g0: Vec<DefaultReal>,
    pub old_source_g1: Vec<DefaultReal>,
    /// `g0` and `g1` as the `Level=2` `Coeff` call left them; the `Level=2`
    /// `MatSet` call reads these (`794-799`, `895-901`).
    pub new_source_g0: Vec<DefaultReal>,
    pub new_source_g1: Vec<DefaultReal>,
    /// `wc` (`764`).
    pub upwind_weight: Vec<DefaultReal>,
    /// `epsi`, `alf`, `dt` (`755`).
    pub epsi: DefaultReal,
    pub dt: DefaultReal,
    pub alf: DefaultReal,
    /// `cAtm` and `dSurf` (`759`), the `kTopCh=-2` atmospheric inputs.
    pub top_atmospheric_concentration: Option<DefaultReal>,
    pub top_surface_distance: Option<DefaultReal>,
    /// `TempO(N)` and `TempN(N)` (`765`).
    pub old_temperature: Option<DefaultReal>,
    pub new_temperature: Option<DefaultReal>,
    /// `ChPar((jS-1)*16+slot, M)` (`765`, `889-890`, `922-923`). The source
    /// indexes the material in the second array dimension, so one record per
    /// species/material pair carries the 20 `ChPar` slots.
    pub source_chpar: Option<Vec<MaterialSpeciesParameters>>,
    /// `TDep((jS-1)*16+slot)` (`766`, `887-890`), the flat reader layout with a
    /// 16-entry stride per species (`INPUT.FOR:1379-1390`).
    pub source_tdep: Option<Vec<DefaultReal>>,
    /// `MatNum` (`766`), one source material number per node.
    pub material_ids: Vec<usize>,
    /// `jS`, the species index `Solute` passes to `MatSet` (`51`, `124`).
    pub source_species: usize,
    /// `lMobIm`, `lBact`, `iDualPor` (`758-759`), the `768-776` water selectors.
    pub mobile_immobile: Vec<bool>,
    pub bacteria: bool,
    pub dual_porosity: u8,
    /// `rBot`, `cBot(jS)`, `cTop(jS)` (`756`, `759`).
    pub bottom_resistance: DefaultReal,
    pub bottom_prescribed: DefaultReal,
    pub top_prescribed: DefaultReal,
    /// `TLevel` (`24`), `lVapor` (`759`) and `rMin` (`41`).
    pub t_level: i32,
    pub vapor: bool,
    pub r_min: DefaultReal,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpeciesRecord {
    pub name: String,
    pub position: usize,
    pub chain_rate_per_hour: DefaultReal,
    pub initial_concentration: DefaultReal,
    pub linear_equilibrium_kd: DefaultReal,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReactionInputs {
    pub equilibrium_mode: Option<UnitValue<bool>>,
    pub fraction_equilibrium: Option<UnitValue<DefaultReal>>,
    pub immobile_water_content: Option<UnitValue<DefaultReal>>,
    pub initial_dissolved_concentration: Option<UnitValue<Vec<DefaultReal>>>,
    pub initial_equilibrium_sorbed_pool: Option<UnitValue<Vec<DefaultReal>>>,
    pub linearity_mode: Option<UnitValue<bool>>,
    pub observation_times: Option<UnitValue<Vec<DefaultReal>>>,
    pub ordered_species: Option<UnitValue<Vec<SpeciesRecord>>>,
    pub reaction_pattern: Option<UnitValue<String>>,
    pub secondary_sorbed_pool: Option<UnitValue<Vec<DefaultReal>>>,
    pub material_species_chpar: Option<UnitValue<Vec<MaterialSpeciesParameters>>>,
    pub moisture_dependency: Option<UnitValue<MoistureDependencyRequest>>,
    pub mass_initialization: Option<UnitValue<bool>>,
    pub equilibrium_initialization: Option<UnitValue<bool>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaterialSpeciesParameters {
    pub species: usize,
    pub material: usize,
    /// Source `ChPar` slots 1..20 for this species/material.
    pub slots: Vec<DefaultReal>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MoistureDependencyRequest {
    pub selector: u8,
    pub table: Vec<DefaultReal>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TransportPropertyInputs {
    pub coordinates: Option<UnitValue<CoordinatesSpec>>,
    pub temperature_history: Option<UnitValue<Vec<DefaultReal>>>,
    pub species_count: Option<UnitValue<usize>>,
    pub transport_switches: Option<UnitValue<SourceSwitches>>,
    pub material_bulk_density: Option<UnitValue<DefaultReal>>,
    pub initial_pressure_head: Option<UnitValue<DefaultReal>>,
    pub initial_pressure_head_profile: Option<UnitValue<Vec<DefaultReal>>>,
    pub initial_temperature_profile: Option<UnitValue<Vec<DefaultReal>>>,
    pub initial_gas_state: Option<UnitValue<String>>,
    pub vapor_state: Option<UnitValue<String>>,
    pub species_1_gas_diffusion: Option<UnitValue<DefaultReal>>,
    pub species_1_henry_coefficient: Option<UnitValue<DefaultReal>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DualDomainRequest {
    pub dynamic_exchange_claimed: bool,
    pub i_dual_por: u8,
    pub water_model: u8,
}

/// One material's `ParD(·,M)` column entries that the active `DualPor` call
/// reads (`WATFLOW.FOR:1036-1068`). Slots a model-6 deck never reaches are
/// optional; the reader fills `ParD(1..9,M)` for water model 6 and
/// `ParD(1..11,M)` for model 7 (`INPUT.FOR:618-634`).
///
/// `thr`/`ths` are deliberately absent: `GenMat` derives them from the same
/// `ParD` column (`thr(M)=Par(1,M)`, `ths(M)=Par(2,M)`, `INPUT.FOR:796-797`),
/// so carrying them here would let a request contradict `ParD(1)`/`ParD(2)`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DualPorMaterial {
    /// `ParD(1,M)`; becomes `thr(M)` in `GenMat` (`INPUT.FOR:796`).
    pub mobile_residual_water: DefaultReal,
    /// `ParD(2,M)`; becomes `ths(M)` in `GenMat` (`INPUT.FOR:797`).
    pub mobile_saturated_water: DefaultReal,
    /// `ParD(6,M)`, the deck's `l` path-length term; `FK` receives it as `BPar`
    /// (`WATFLOW.FOR:1050`, `MATERIAL.FOR:29`). Unread when `iDualPor=1`.
    pub mobile_tortuosity: Option<DefaultReal>,
    /// `ParD(7,M)`, immobile-region residual water content.
    pub immobile_residual_water: DefaultReal,
    /// `ParD(8,M)`, immobile-region saturated water content.
    pub immobile_saturated_water: DefaultReal,
    /// `ParD(9,M)`: the first-order transfer coefficient of `1040` when
    /// `iDualPor=1`, and the immobile-region van Genuchten `Alfa` of `1047`
    /// when `iDualPor=2`.
    pub immobile_alpha: DefaultReal,
    /// `ParD(10,M)`, the immobile-region van Genuchten `n` (`WATFLOW.FOR:1048`,
    /// `MATERIAL.FOR:359`). Unread when `iDualPor=1`.
    pub immobile_n: Option<DefaultReal>,
    /// `ParD(11,M)`, the immobile-region saturated conductivity, `FK`'s `Ks`
    /// (`WATFLOW.FOR:1049`, `MATERIAL.FOR:28`). Unread when `iDualPor=1`.
    pub immobile_saturated_conductivity: Option<DefaultReal>,
}

/// The primitive state the active dynamic dual-porosity call receives
/// (`WATFLOW.FOR:43-46`, argument list `1027-1029`). The routine's three
/// outputs - `SinkIm`, `ThNewIm` and `WTransf` - are not representable here:
/// `ThOldIm` is the only immobile-water input, because `DualPor` overwrites
/// `ThNewIm` at `1068` and nothing reads it before the routine returns, and
/// `ThOldIm(i)=ThNewIm(i)` belongs to `Update` (`WATFLOW.FOR:1112`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DualPorPrimitiveState {
    /// `x` (`1029`), native node coordinates in source node order.
    pub coordinates: Vec<DefaultReal>,
    /// `ThOld` (`1027`), the mobile water content at the step's old level.
    pub old_water: Vec<DefaultReal>,
    /// `hNew` (`1029`), the pressure head `WatFlow` advances the step from.
    pub new_pressure_head: Vec<DefaultReal>,
    /// `ThOldIm` (`1028`), immobile water content when `DualPor` is called.
    pub old_immobile_water: Vec<DefaultReal>,
    /// `MatNum` (`1027`), the one-based `ParD` column of each node.
    pub material_ids: Vec<usize>,
    /// `ParD(·,M)` (`1028`), one record per material.
    pub materials: Vec<DualPorMaterial>,
    /// `hCritA` (`1029`), the atmospheric-pressure ceiling of the `1054-1055`
    /// guard. Required, never defaulted: neither `src_reference/INPUT.FOR:67`,
    /// `102-103` nor any patched reader (there is no `patch/INPUT.f90` in this
    /// tree, and the pinned patch rewrites only `HYDRUS.FOR`/`TIME.FOR`) reads it
    /// from a data file.
    pub critical_atmospheric_head: DefaultReal,
    /// `dt` (`1028`), the water step's time step.
    pub dt: DefaultReal,
    /// `iModel` (`1028`) as `WatFlow` receives it, i.e. after the reader's
    /// `iModel=0` remap of water models 6 and 7 (`INPUT.FOR:619-624`).
    pub hydraulic_model: i32,
    /// `iDualPor` (`1027`), 1 or 2; the call guard at `43` is `iDualPor.gt.0`.
    pub dual_porosity: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NonequilibriumInputs {
    pub dual_domain: Option<UnitValue<DualDomainRequest>>,
    pub dynamic_exchange_sink: Option<UnitValue<Vec<DefaultReal>>>,
    pub dual_porosity_exchange_state: Option<UnitValue<DualPorPrimitiveState>>,
    pub initial_dual_immobile_water_content: Option<UnitValue<Vec<DefaultReal>>>,
    pub initial_water_content: Option<UnitValue<Vec<DefaultReal>>>,
}

/// The primitive state `SetSSnk` receives (`SOLUTE.FOR:94-96` call, dummy list
/// `SINK.FOR:138-139`). Field order is the dummy-argument order, and every field
/// is a value the routine reads: the one array it writes, `SinkS` (`Solute`'s
/// `sSink`), is an output of the port and is deliberately absent here.
///
/// Nothing here is derived. `Sink`, `Beta` and `x` come from the water-flow and
/// root producers (`SetSnk` - `HYDRUS.FOR:352-355` and `482-487`, `SetRG`, the
/// `Beta.in` reader), `Conc(jS,·)` is `Solute`'s own state, and `OmegaW`, `SPot`,
/// `rKM`, `cMin`, `OmegaS` and `cRootMax` are `SinkIn` state read at
/// `INPUT.FOR:1083-1085` and `1122-1126`. `INPUT.FOR:1696-1697` defaults `SPot=0.`
/// and `OmegaW=1.` and `INPUT.FOR:1695` defaults `OmegaS=1.`, but `rKM` and `cMin`
/// are never defaulted anywhere in `src_reference`; the port therefore requires all
/// five explicitly instead of reusing a reader default as a substitute.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RootUptakePrimitiveState {
    /// `jS`, the species index `Solute` passes from its `do 18 jS=1,NS` loop
    /// (`SOLUTE.FOR:51`); it selects `Conc(jS,·)` and guards the unit-78 record.
    pub species: usize,
    /// `NS`, the species count of the same loop.
    pub species_count: usize,
    /// `t` (`141`: `double precision t`), the only double-precision dummy. The
    /// routine reads it once, in the `write(78,100)` of the column-6 continuation
    /// `SINK.FOR:217`.
    pub time: DoubleReal,
    /// `x` (`138`), native node coordinates in source node order.
    pub coordinates: Vec<DefaultReal>,
    /// `Beta` (`138`), the root-length distribution; `Beta(i)<=0.` selects the
    /// `212-214` branch that zeroes `SinkS(i)`.
    pub root_distribution: Vec<DefaultReal>,
    /// `Sink` (`138`), the root water uptake rate density produced by `SetSnk`.
    pub root_water_sink: Vec<DefaultReal>,
    /// `Conc(jS,·)` (`138`) as `Solute` holds it at `SOLUTE.FOR:95`, i.e. before
    /// the isotherm iteration of `99` starts.
    pub species_concentration: Vec<DefaultReal>,
    /// `OmegaW` (`139`), the actual-to-potential transpiration ratio `SetSnk`
    /// computes at `SINK.FOR:130`.
    pub omega_water: DefaultReal,
    /// `cRootMax` (`139`). The call passes the element `cRootMax(jS)`
    /// (`SOLUTE.FOR:96`) into a scalar dummy, so this is one species' ceiling,
    /// not the `NS` array - the same element-versus-array distinction as `FH`'s
    /// `real Par(10)` in 18.7.3.
    pub species_root_max_concentration: DefaultReal,
    /// `lActRSU` (`139`), active root solute uptake. Read from the `Site.dat`
    /// control record only for a version-4 deck (`INPUT.FOR:36-38`) and forced
    /// false for a species chain (`INPUT.FOR:1122`); default false
    /// (`INPUT.FOR:1680`). It selects `nStep` at `SINK.FOR:165-166`.
    pub active_uptake: bool,
    /// `OmegaS` (`139`), the solute stress index of `SINK.FOR:166`, `205-206`.
    pub solute_stress_index: DefaultReal,
    /// `SPot` (`139`), the potential root solute uptake of `SINK.FOR:194`.
    pub potential_uptake: DefaultReal,
    /// `rKM` (`139`), the Michaelis-Menten constant of `SINK.FOR:196`, `207-208`.
    pub michaelis_constant: DefaultReal,
    /// `cMin` (`139`), the concentration floor of `SINK.FOR:189`.
    pub minimum_concentration: DefaultReal,
}

/// The primitive state `Exclusion` receives (`HYDRUS.FOR:390-391` call, dummy list
/// `SOLUTE.FOR:1713-1714`), restricted to the values the routine actually reads.
///
/// Three omissions are deliberate and must not be "restored":
/// * `ThOld`/`vOld` and `ThNew(i>1)`/`vNew(i>1)` are write-only (`1772-1777`);
///   the routine reads only `ThNew(1)` (`1742`, `1744`) and `vNew(1)` (`1759`,
///   `1762`), so carrying the other node values would be dead input.
/// * `Par(3,1)` is read into `alpha` at `1729` but its only reader is `1766`, and
///   `1765-1769` is dead because its sole consumer, `1770`, carries `c` in column 1.
///   A dead store gets no input field.
/// * `ChPar(4,1)` is both read (`1722`) and zeroed (`1724`); the value after the
///   call is an output of the port, so only the incoming value is here.
///
/// `M=1` is hard-coded at `1721`, so the routine is single-material: `Par(·,1)` and
/// `ChPar(·,1)` are the only columns it can reach.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SizeExclusionPrimitiveState {
    /// `NumNP` (`1713`), the loop bound of the `1772-1777` write loop.
    pub nodes: usize,
    /// `Par(1,1)`, `thr` (`1725`).
    pub mobile_residual_water: DefaultReal,
    /// `Par(2,1)`, `ths` (`1726`).
    pub mobile_saturated_water: DefaultReal,
    /// `Par(4,1)`, `vgn1` (`1730`) and the base of `vgn2`/`vgm1`/`vgm2`
    /// (`1732-1734`).
    pub mobile_n: DefaultReal,
    /// `Par(6,1)`, `xL` (`1731`), the Mualem `l` path-length exponent of `1758`.
    /// Every analytic water model fills `Par(6,M)` (`INPUT.FOR:611-629`,
    /// `NPar>=6`), so this is always deck-supplied state.
    pub mobile_tortuosity: DefaultReal,
    /// `ChPar(4,1)`, `th_c` (`1722`): the water content from which colloids are
    /// excluded. Same `ChPar` slot `MatSet` subtracts from both waters.
    pub colloid_exclusion_water_content: DefaultReal,
    /// `ThNew(1)` (`1742`, `1744`). `HYDRUS.FOR:384-389` copies `ThOld` into
    /// `ThNew` immediately before the call, so this is the old-level water content.
    pub first_node_new_water: DefaultReal,
    /// `vNew(1)` (`1759`, `1762`), likewise just copied from `vOld` at `385`.
    pub first_node_new_velocity: DefaultReal,
}

/// Selector and primitive-state group for the two source-coupling consumers.
/// `control_switches` carries the two `Site.dat` guard logicals once (merged into
/// `SourceSwitches` by the adapter), so a request cannot assert a guard value in a
/// bundle that contradicts the control record and reach a call the source forbids.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SourceCouplingInputs {
    /// `lWat` and `SinkF` only, i.e. the subset of the `INPUT.FOR:31` control read
    /// that guards `Exclusion` and `SetSSnk`. Parsed into the shared
    /// `SourceSwitches` type so each flag keeps exactly one home.
    pub control_switches: Option<UnitValue<SourceSwitches>>,
    pub root_uptake_state: Option<UnitValue<RootUptakePrimitiveState>>,
    pub size_exclusion_state: Option<UnitValue<SizeExclusionPrimitiveState>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BiologyModeRequest {
    pub i_bact: u8,
    pub filtration: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BiologyParameters {
    pub blocking_i_psi: Option<u8>,
    pub grain_diameter: Option<DefaultReal>,
    pub particle_diameter: Option<DefaultReal>,
    pub attachment_capacity_1: Option<DefaultReal>,
    pub attachment_rate_1: Option<DefaultReal>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BiologyInputs {
    pub mode: Option<UnitValue<BiologyModeRequest>>,
    pub parameters: Option<UnitValue<BiologyParameters>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RequestFields {
    pub transport: TransportInputs,
    pub reactions: ReactionInputs,
    pub transport_properties: TransportPropertyInputs,
    pub nonequilibrium: NonequilibriumInputs,
    pub biology: BiologyInputs,
    pub source_coupling: SourceCouplingInputs,
}

impl Default for RequestFields {
    fn default() -> Self {
        Self {
            transport: TransportInputs {
                temporal_weighting_epsi: None,
                time_interval: None,
                dispersivity: None,
                molecular_diffusion: None,
                upstream_weighting: None,
                upper_boundary_code: None,
                lower_boundary_code: None,
                flow_orientation: None,
                atmospheric_concentration: None,
                atmospheric_surface_distance: None,
                flux_concentration: None,
                vapor_lower_boundary_exception: None,
                endpoint_state: None,
            },
            reactions: ReactionInputs {
                equilibrium_mode: None,
                fraction_equilibrium: None,
                immobile_water_content: None,
                initial_dissolved_concentration: None,
                initial_equilibrium_sorbed_pool: None,
                linearity_mode: None,
                observation_times: None,
                ordered_species: None,
                reaction_pattern: None,
                secondary_sorbed_pool: None,
                material_species_chpar: None,
                moisture_dependency: None,
                mass_initialization: None,
                equilibrium_initialization: None,
            },
            transport_properties: TransportPropertyInputs {
                coordinates: None,
                temperature_history: None,
                species_count: None,
                transport_switches: None,
                material_bulk_density: None,
                initial_pressure_head: None,
                initial_pressure_head_profile: None,
                initial_temperature_profile: None,
                initial_gas_state: None,
                vapor_state: None,
                species_1_gas_diffusion: None,
                species_1_henry_coefficient: None,
            },
            nonequilibrium: NonequilibriumInputs {
                dual_domain: None,
                dynamic_exchange_sink: None,
                dual_porosity_exchange_state: None,
                initial_dual_immobile_water_content: None,
                initial_water_content: None,
            },
            biology: BiologyInputs {
                mode: None,
                parameters: None,
            },
            source_coupling: SourceCouplingInputs {
                control_switches: None,
                root_uptake_state: None,
                size_exclusion_state: None,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpeciesInput {
    pub name: String,
    pub chpar: Vec<DefaultReal>,
    pub initial_concentration: Vec<DefaultReal>,
    pub sorb: Explicit<Vec<DefaultReal>>,
    pub sorb2: Explicit<Vec<DefaultReal>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CompatibilityInput {
    pub schema_version: String,
    pub request_sha256: String,
    pub input_sha256: Vec<(String, String)>,
    pub units: Vec<(String, String)>,
    pub fields: RequestFields,
    pub coordinates: Explicit<Vec<DefaultReal>>,
    pub material_ids: Explicit<Vec<usize>>,
    pub species: Vec<SpeciesInput>,
    pub old_water: Explicit<Vec<DefaultReal>>,
    pub new_water: Explicit<Vec<DefaultReal>>,
    pub temperature: Explicit<Vec<DefaultReal>>,
    pub dual_immobile_water: Explicit<Vec<DefaultReal>>,
    pub switches: SourceSwitches,
    pub upper_boundary: Option<BoundaryForm>,
    pub lower_boundary: Option<BoundaryForm>,
    pub source_indices: Explicit<SourceIndices>,
}

impl CompatibilityInput {
    pub(crate) fn validate_dimensions(&self) -> Result<(), KernelError> {
        let species_count = self.species.len().max(
            self.fields
                .transport_properties
                .species_count
                .as_ref()
                .and_then(|v| match &v.value {
                    Explicit::Available(n) => Some(*n),
                    _ => None,
                })
                .unwrap_or(0),
        );
        self.switches.validate(species_count)?;
        if let Explicit::Available(indices) = &self.source_indices {
            if let Some(nodes) = self
                .fields
                .transport_properties
                .coordinates
                .as_ref()
                .and_then(|v| match &v.value {
                    Explicit::Available(c) => Some(c.nodes),
                    Explicit::Unavailable(_) => None,
                })
            {
                indices.validate(nodes)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CompatibilityState {
    pub time: DefaultReal,
    pub concentration: Vec<Vec<DefaultReal>>,
    pub sorb: Vec<Explicit<Vec<DefaultReal>>>,
    pub sorb2: Vec<Explicit<Vec<DefaultReal>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CompatibilityWorkspace {
    pub lower: Vec<DoubleReal>,
    pub diagonal: Vec<DoubleReal>,
    pub upper: Vec<DoubleReal>,
    pub rhs: Vec<DoubleReal>,
    pub source_terms: Vec<Vec<DefaultReal>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TraceObservation {
    pub species: usize,
    pub native_node: Option<usize>,
    pub checkpoint: String,
    pub default_real: Option<DefaultReal>,
    pub double_real: Option<DoubleReal>,
    pub unavailable_reason: Option<String>,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn precision_boundaries_match_source_kinds() {
        let x = DefaultReal::from_input(1.0 / 3.0);
        assert_eq!(x.0, (1.0_f64 / 3.0) as f32);
        assert_eq!(DoubleReal(-2.0).accepted_concentration(), DefaultReal(0.0));
        assert_eq!(
            DoubleReal(1.0 / 3.0).accepted_concentration().0,
            (1.0_f64 / 3.0) as f32
        );
    }
    #[test]
    fn source_indices_are_one_based_bijection() {
        assert!(SourceIndices {
            canonical_to_native: vec![3, 2, 1]
        }
        .validate(3)
        .is_ok());
        assert!(SourceIndices {
            canonical_to_native: vec![1, 1]
        }
        .validate(2)
        .is_err());
    }
    #[test]
    fn switches_guard_unreachable_and_invalid_modes() {
        let switches = SourceSwitches {
            upstream: true,
            artificial_dispersion: true,
            ..SourceSwitches::default()
        };
        assert!(switches.validate(2).is_ok());
        assert!(SourceSwitches {
            linear: false,
            ..switches
        }
        .validate(1)
        .is_ok());
        assert!(SourceSwitches {
            dual_domain_model: Some(8),
            ..switches
        }
        .validate(1)
        .is_err());
        assert!(SourceSwitches {
            dual_domain_model: Some(6),
            mobile_immobile: true,
            ..switches
        }
        .validate(1)
        .is_err());
    }
    #[test]
    fn dual_porosity_materials_derive_thr_and_ths_from_the_pard_column() {
        // GenMat (INPUT.FOR:796-797) assigns thr(M)=Par(1,M) and
        // ths(M)=Par(2,M), so the exchange bundle carries the ParD column only:
        // the pair the exchange reads is the mobile pair it stores.
        let model6 = DualPorMaterial {
            mobile_residual_water: DefaultReal(0.08),
            mobile_saturated_water: DefaultReal(0.43),
            mobile_tortuosity: None,
            immobile_residual_water: DefaultReal(0.01),
            immobile_saturated_water: DefaultReal(0.12),
            immobile_alpha: DefaultReal(0.1),
            immobile_n: None,
            immobile_saturated_conductivity: None,
        };
        let state = DualPorPrimitiveState {
            coordinates: vec![DefaultReal(0.0), DefaultReal(0.25)],
            old_water: vec![DefaultReal(0.3), DefaultReal(0.3)],
            new_pressure_head: vec![DefaultReal(-100.0), DefaultReal(-100.0)],
            old_immobile_water: vec![DefaultReal(0.05), DefaultReal(0.05)],
            material_ids: vec![1, 1],
            materials: vec![model6.clone()],
            critical_atmospheric_head: DefaultReal(-1.0e10),
            dt: DefaultReal(0.1),
            hydraulic_model: 0,
            dual_porosity: 1,
        };
        let material = &state.materials[0];
        assert_eq!(material.mobile_residual_water, DefaultReal(0.08));
        assert_eq!(material.mobile_saturated_water, DefaultReal(0.43));
        assert_eq!(
            state.critical_atmospheric_head,
            DefaultReal(-1.0e10),
            "hCritA is an explicit primitive, never a reader default"
        );
        assert!(model6.mobile_tortuosity.is_none() && model6.immobile_n.is_none());
    }

    #[test]
    fn compatibility_input_preserves_unavailable_fields_and_orientation() {
        let unavailable_usize = Explicit::Unavailable(UnavailableValue {
            availability: "inactive".into(),
            reason: "source call is commented".into(),
        });
        let unavailable_real = Explicit::Unavailable(UnavailableValue {
            availability: "inactive".into(),
            reason: "source call is commented".into(),
        });
        let input = CompatibilityInput {
            schema_version: "solute-request-v1".into(),
            request_sha256: "0".repeat(64),
            input_sha256: vec![],
            units: vec![],
            fields: RequestFields::default(),
            coordinates: Explicit::Available(vec![DefaultReal(0.0), DefaultReal(1.0)]),
            material_ids: unavailable_usize,
            species: vec![],
            old_water: unavailable_real.clone(),
            new_water: unavailable_real.clone(),
            temperature: unavailable_real.clone(),
            dual_immobile_water: unavailable_real,
            switches: SourceSwitches::default(),
            upper_boundary: Some(BoundaryForm::Atmospheric),
            lower_boundary: Some(BoundaryForm::ZeroGradient),
            source_indices: Explicit::Available(SourceIndices::identity(2)),
        };
        assert!(input.validate_dimensions().is_ok());
        assert_eq!(
            input.source_indices,
            Explicit::Available(SourceIndices {
                canonical_to_native: vec![1, 2]
            })
        );
    }
}

#[path = "output_compat.rs"]
#[allow(dead_code)] // wired by 20.4.1
pub(crate) mod output_compat;
