# M6 vapor G22.3 contract: atmospheric switching and meteorology

Status: **PROPOSED (orchestration 31, task 31.1.3)** for GATE A approval.
Repo: hyd.rs (branch `feat/vapor-flux-fixedhead`). Tracked planning (decision D-5).
Governing plan: [EXECUTION_PLAN_31.md](../EXECUTION_PLAN_31.md).
Governing goals: [GOALS.md](../GOALS.md) G22.3 ("Atmospheric water switching with vapor; lMeteo=true; Penman-Monteith potential evaporation from met input"), coordinated with the radiation term split of G20 (decision D-4).

---

## 1. Scope and boundaries

### In scope
1. **Meteorological series input**: Ingestion and validation of time-variable meteorological records (temperatures $T_{\max}, T_{\min}$, solar/net radiation $R_{\text{ad}}$, wind speed $W$, relative humidity $RH$, sunshine hours $n$).
2. **Site & surface properties**: Geodetic site coordinates (latitude, altitude), measurement heights ($z_w, z_t$), radiation coefficients ($a_s, b_s, a_c, b_c, a_1, b_1$), surface albedo ($\alpha$), and crop/cover parameters.
3. **Net radiation terms ($R_n, R_{ns}, R_{nl}$)**: Computation of net shortwave ($R_{ns}$), net longwave ($R_{nl}$), and total net radiation ($R_n$) in Rust under strict source operation order and precision rules (ADR 0010, ADR 0015).
4. **Evapotranspiration & partition**: Aerodynamic resistance ($r_{aa}$), psychrometric constant ($\gamma$), slope of saturation vapor pressure curve ($\Delta$), and Penman-Monteith combination evapotranspiration ($ET_{\text{comb}}$), partitioned into potential soil evaporation ($E_p / r_{\text{Soil}}$) and potential crop transpiration ($T_p / r_{\text{Root}}$).
5. **Atmospheric water switching with vapor coupling**: Integration of the atmospheric upper boundary (`UpperBoundaryMode::{AtmosphericFlux, PondedHead}`) with active vapor transport (`lVapor=true`), preserving two-pass water-velocity-heat convergence and atomic retry semantics.
6. **Provenance & parity**: Pinned Fortran reference execution (`lMeteo=true`, `lVapor=true`) providing deterministic verification trajectories.

### Out of scope (explicit)
1. **Full surface energy balance (`lEnBal=true`, `UpdateEnergy`)**: The energy update and surface temperature iteration remain deferred to a dedicated orchestration following G20 split (decision D-4).
2. **Temperature-dependent liquid water flow (`lWDep=true`)**: Deferred to G23.
3. **Enhancement factor off (`iEnhanc=0`) and Campbell thermal conductivity (`iCampbell=1`)**: Deferred to G22.5/G22.6.
4. **Unsupported lower boundary modes**: Seepage face (`SeepF`), drainage table (`qDrain`), and deep drainage (`qGWLF`) remain rejected with triggers per `M6_VAPOR_G22_2_ACCEPTANCE.md`.

---

## 2. Input specifications and conventions

### 2.1 Meteorological time series records

| Parameter | Symbol | Source name | Units | Valid range | Physical meaning / notes |
|---|---|---|---|---|---|
| Time | $t_{\text{atm}}$ | `tAtm` | T (days / hours) | $t_{\text{atm}} \ge t_{\text{init}}$ | Monotonically increasing event timestamp |
| Solar / Net Radiation | $R_{\text{ad}}$ | `Rad` | $\text{MJ}/\text{m}^2/\text{d}$ | $R_{\text{ad}} \ge 0$ | Global solar or net radiation at ground level |
| Maximum Temperature | $T_{\max}$ | `TMax` | ${}^\circ\text{C}$ | $-60.0 \le T_{\max} \le 70.0$ | Daily or interval maximum air temperature |
| Minimum Temperature | $T_{\min}$ | `TMin` | ${}^\circ\text{C}$ | $-60.0 \le T_{\min} \le T_{\max}$ | Daily or interval minimum air temperature |
| Relative Humidity | $RH$ | `RHMean` | % or kPa | $0.0 \le RH \le 100.0$ (%) | Relative humidity (%) or vapor pressure (kPa) based on `iRelHum` |
| Wind Speed | $W$ | `Wind_kmd` | $\text{km}/\text{d}$ | $W \ge 0$ | Mean horizontal wind speed at measurement height $z_w$ |
| Sunshine Hours | $n$ | `SunHours` | hours | $0 \le n \le 24$ | Actual daily sunshine duration |

### 2.2 Site metadata and atmospheric constants

| Parameter | Symbol | Source name | Units | Default | Fortran site | Valid range / Constraints |
|---|---|---|---|---|---|---|
| Latitude | $\Phi$ | `Latitude`, `rLat` | degrees | 0.0 | `INPUT.FOR:971` | $-90.0 \le \Phi \le 90.0$ (degrees + minutes format) |
| Altitude | $z_{\text{alt}}$ | `Altitude`, `rAlt` | m | 0.0 | `INPUT.FOR:971` | $-500.0 \le z_{\text{alt}} \le 9000.0$ |
| Solar Constant | $I_{sc}$ | `SC` | $\text{MJ}/\text{m}^2/\text{d}$ | 118.08 | `TIME.FOR:677` | Pinned physical constant $118.08\,\text{MJ}\,\text{m}^{-2}\,\text{d}^{-1}$ |
| Shortwave Coeff A | $a_s$ | `ShortWaveRadA` | [-] | 0.25 | `INPUT.FOR:948` | $0.0 \le a_s \le 1.0$ (Angstrom coefficient) |
| Shortwave Coeff B | $b_s$ | `ShortWaveRadB` | [-] | 0.50 | `INPUT.FOR:950` | $0.0 \le b_s \le 1.0, a_s + b_s \le 1.0$ |
| Longwave Coeff A | $a_c$ | `LongWaveRadA` | [-] | 0.90 | `INPUT.FOR:953` | Cloudiness factor parameter |
| Longwave Coeff B | $b_c$ | `LongWaveRadB` | [-] | 0.10 | `INPUT.FOR:954` | Cloudiness factor parameter |
| Emissivity Coeff A1 | $a_1$ | `LongWaveRadA1` | [-] | 0.34 | `INPUT.FOR:955` | Emissivity correlation coefficient |
| Emissivity Coeff B1 | $b_1$ | `LongWaveRadB1` | $\text{kPa}^{-0.5}$ | -0.139 | `INPUT.FOR:956` | Emissivity correlation coefficient |
| Wind Measurement Height | $z_w$ | `WindHeight` | cm | 200.0 | `INPUT.FOR:957` | $z_w > 0.667 \cdot h_{\text{crop}}$ |
| Temperature Measurement Height | $z_t$ | `TempHeight` | cm | 190.0 | `INPUT.FOR:958` | $z_t > 0.667 \cdot h_{\text{crop}}$ |
| Surface Albedo | $\alpha$ | `Albedo` | [-] | 0.23 | `INPUT.FOR:994,1017` | $0.0 \le \alpha \le 1.0$ |
| Crop Height | $h_{\text{crop}}$ | `CropHeight` | cm | 0.1 | `INPUT.FOR:994, TIME.FOR:739` | $0.667 \cdot h_{\text{crop}} < \min(z_w, z_t)$ |
| Extinction Coefficient | $k_{\text{ext}}$ | `rExtinct` | [-] | 0.463 | `INPUT.FOR:989` | Canopy radiation extinction coefficient |

---

## 3. Fortran provenance mapping table

The following table establishes the exact provenance for each meteorological and atmospheric input and operation between the pinned Fortran reference (`src_reference/`) and the Rust architecture.

| Concept / Field | Fortran Routine & Source Anchor | Fortran File:Line | Rust Destination / Mapping | Functional Role |
|---|---|---|---|---|
| `lMeteo` Flag | `INPUT.FOR` (Record 3, iVer=4) | `src_reference/INPUT.FOR:38` | `WaterProductConfig::meteo` / option | Enables reading meteorological data and Penman-Monteith ET |
| Meteo File Unit 33 | `HYDRUS.FOR` (`open(33)`) | `src_reference/HYDRUS.FOR:186-189` | Case runner / `Meteo.in` parser | Opens input file `Meteo.in` when `lMeteo=true` |
| Meteo Output Unit 43 | `HYDRUS.FOR` (`open(43)`) | `src_reference/HYDRUS.FOR:190-192` | Tool output / fixture log | Formats output file `Meteo.out` when `lPrint=true` |
| Configuration Reader | `subroutine MeteoIn` | `src_reference/INPUT.FOR:934-1066` | `meteo::config::MeteoConfig` | Reads site coords, heights, radiation/crop coefficients |
| Latitude / Altitude | `MeteoIn` (`Latitude, Altitude`) | `src_reference/INPUT.FOR:970-971` | `MeteoConfig::latitude`, `altitude` | Geodetic location for solar angles and atmospheric pressure |
| Rad Coefficients | `MeteoIn` (`ShortWaveRadA/B`) | `src_reference/INPUT.FOR:972-978` | `meteo::radiation::RadiationParams` | Angstrom coefficients, cloudiness and emissivity factors |
| Measurement Heights | `MeteoIn` (`WindHeight, TempHeight`) | `src_reference/INPUT.FOR:979-980` | `MeteoConfig::wind_height`, `temp_height` | Anemometer and thermometer elevation above ground |
| Crop & Cover Params | `MeteoIn` (`CropHeight, Albedo, LAI`) | `src_reference/INPUT.FOR:981-1020` | `meteo::crop::CropConfig` | Surface reflection, vegetation cover, and roughness |
| Meteo Time Dispatch | `subroutine Meteo(iKod, ...)` | `src_reference/TIME.FOR:317-430` | `meteo::driver::MeteoDriver` | Dispatches lifecycle events: 1=init, 2=step update, 3=print step |
| Daily / Periodic Step | `subroutine SetMeteo(...)` | `src_reference/TIME.FOR:443-626` | `meteo::evaporation::calculate_meteo_step` | Processes records, computes $R_n$, $ET_{\text{comb}}$, $E_p$, $T_p$ |
| Record Ingestion | `SetMeteo` (`read(33,*)`) | `src_reference/TIME.FOR:484-497` | `meteo::series::MeteoRecord` | Parses `tAtm, Rad, TMax, TMin, RHMean, Wind_kmd, SunHours` |
| Wind Speed Conversion | `SetMeteo` (`Wind_ms = Wind_kmd/86.4`) | `src_reference/TIME.FOR:515` | `meteo::evaporation::wind_km_day_to_m_s` | Converts daily travel distance ($\text{km}/\text{d}$) to velocity ($\text{m}/\text{s}$) |
| Vapor Pressure (Tetens) | `SetMeteo` (`Ea_TMax, Ea_TMin, EaDew`) | `src_reference/TIME.FOR:522-531` | `meteo::radiation::saturation_vapor_pressure` | Tetens formula for saturation and actual vapor pressure |
| Extraterrestrial Radiation | `subroutine RadGlobal` | `src_reference/TIME.FOR:672-689` | `meteo::radiation::extraterrestrial_radiation` | Computes solar declination, sunset hour angle $\omega$, and $R_a$ |
| Latitude Deg/Min Conversion | `RadGlobal` (`Lat1 = (Lat - aint(Lat))*5/3 + aint(Lat)`) | `src_reference/TIME.FOR:679-680` | `meteo::radiation::parse_latitude_deg_min` | Converts legacy degrees-and-minutes decimal format to radians |
| Cloudiness Factor | `subroutine Cloudiness` | `src_reference/TIME.FOR:698-731` | `meteo::radiation::cloudiness_factor` | Computes cloud cover ratio and relative sunshine duration |
| Net Shortwave Rad | `SetMeteo` (`Rns = (1 - Albedo)*Rad`) | `src_reference/TIME.FOR:572` | `meteo::radiation::net_shortwave` | Net incoming solar radiation after surface reflection |
| Net Longwave Rad | `subroutine RadLongNet` | `src_reference/TIME.FOR:771-781` | `meteo::radiation::net_longwave` | Stefan-Boltzmann radiation with cloudiness & emissivity |
| Net Radiation Budget | `SetMeteo` (`Rn = Rns - Rnl`) | `src_reference/TIME.FOR:579` | `meteo::radiation::net_radiation` | Total surface energy available for sensible and latent fluxes |
| Aerodynamic Resistance | `subroutine Aero` | `src_reference/TIME.FOR:630-667` | `meteo::evaporation::aerodynamic_terms` | Boundary layer resistance, psychrometric $\gamma$, slope $\Delta$ |
| Measurement Height Check | `Aero` (`if (dl0 >= WindHeight ...) goto 901`) | `src_reference/TIME.FOR:638-639, 663` | `MeteoError::CropHeightExceedsMeasurementHeight` | Enforces zero-plane displacement height check (Error 33) |
| Penman-Monteith ET | `SetMeteo` (`ETcomb = max(0, RadTerm + AeroTerm)`) | `src_reference/TIME.FOR:591-595` | `meteo::evaporation::penman_monteith_et` | Total potential evapotranspiration ($\text{mm}/\text{d}$) |
| Potential Evap & Transp | `SetMeteo` (`EvapP = ETcomb*(1-SCF); TransP = ...`) | `src_reference/TIME.FOR:601-602` | `meteo::evaporation::partition_et` | Splits $ET$ into soil evaporation demand and crop transpiration |
| Atmospheric Boundary Switch | `SetBC` (`rTop = -abs(Prec)`) | `src_reference/TIME.FOR:130-131` | `atmospheric::advance_atmospheric_step` | Directs precipitation and potential evaporation to boundary |
| Step Application in Hydrus | `HYDRUS.FOR` (`call Meteo(2, ...)`) | `src_reference/HYDRUS.FOR:592-609` | `atmospheric.rs:55` (`advance_atmospheric_step`) | Advances atmospheric boundary step when simulation hits event |
| Error Message 32 | `ErrorOut` (`cErr(32)='Error when reading ... Meteo.in'`) | `src_reference/HYDRUS.FOR:829` | `MeteoError::InputFileReadError` | Raised on malformed or truncated `Meteo.in` input deck |
| Error Message 33 | `ErrorOut` (`cErr(33)='Crop Height must be smaller ...'`) | `src_reference/HYDRUS.FOR:830-831` | `MeteoError::CropHeightExceedsMeasurementHeight` | Raised when crop zero displacement reaches measurement height |

---

## 4. Mathematical formulations and precision rules (ADR 0010 & ADR 0015)

In accordance with ADR 0010 (preserve source operation order and precision) and ADR 0015 (precision-faithful ports), all meteorological equations must replicate the exact calculation sequence of the reference Fortran source. Intermediate operations in kernel-level routines retain single-precision IEEE 754 float arithmetic (`f32`) or exact IEEE 754 `f64` conversions where appropriate, avoiding mathematical rearrangements that alter floating-point rounding.

### 4.1 Extraterrestrial radiation ($R_a$)
1. Latitude conversion:
   $$\text{Lat}_1 = (\Phi - \operatorname{aint}(\Phi)) \cdot \frac{5}{3} + \operatorname{aint}(\Phi), \quad \text{Lat}_2 = \text{Lat}_1 \cdot \frac{\pi}{180}$$
2. Solar declination ($\delta$):
   $$\delta = 0.4093 \cdot \sin\left(\frac{2\pi}{365} \cdot \text{DayNo} - 1.39\right)$$
3. Sunset hour angle ($\omega$):
   $$\omega = \arccos(-\tan(\delta) \cdot \tan(\text{Lat}_2))$$
4. Geometric terms:
   $$x_x = \sin(\delta) \cdot \sin(\text{Lat}_2), \quad y_y = \cos(\delta) \cdot \cos(\text{Lat}_2)$$
5. Relative earth-sun distance ($d_r$):
   $$d_r = 1.0 + 0.033 \cdot \cos\left(\frac{2\pi}{365} \cdot \text{DayNo}\right)$$
6. Extraterrestrial radiation ($R_a$):
   $$R_a = \frac{I_{sc}}{\pi} \cdot d_r \cdot (\omega \cdot x_x + \sin(\omega) \cdot y_y)$$
   where $I_{sc} = 118.08\,\text{MJ}\,\text{m}^{-2}\,\text{d}^{-1}$ and $\pi = 3.141592654$.

### 4.2 Net shortwave radiation ($R_{ns}$) and net longwave radiation ($R_{nl}$)
1. Solar radiation from Angstrom formula (when `iRadiation=0`):
   $$R_{\text{ad}} = R_a \cdot \left(a_s + b_s \cdot \frac{n}{N}\right), \quad N = \frac{24}{\pi} \cdot \omega$$
2. Net shortwave radiation:
   $$R_{ns} = (1.0 - \alpha) \cdot R_{\text{ad}}$$
3. Saturation vapor pressure at minimum and maximum temperature (Tetens formula):
   $$e_a(T) = 0.6108 \cdot \exp\left(\frac{17.27 \cdot T}{T + 237.3}\right)$$
   $$e_a^{\text{dew}} = \begin{cases} 
   \frac{RH}{\frac{50.0}{e_a(T_{\min})} + \frac{50.0}{e_a(T_{\max})}} & \text{if } i_{\text{RelHum}} = 0 \\
   RH & \text{if } i_{\text{RelHum}} = 1 
   \end{cases}$$
4. Stefan-Boltzmann longwave term ($\sigma_{SB}$):
   $$\sigma_{SB} = 2.45 \times 10^{-9} \cdot \left((T_{\max} + 273.16)^4 + (T_{\min} + 273.16)^4\right)$$
5. Net emissivity:
   $$\varepsilon_{\text{net}} = a_1 + b_1 \cdot \sqrt{e_a^{\text{dew}}}$$
6. Net longwave radiation ($R_{nl}$):
   $$R_{nl} = \sigma_{SB} \cdot \text{CloudF} \cdot \varepsilon_{\text{net}}$$
7. Net radiation budget ($R_n$):
   $$R_n = R_{ns} - R_{nl}$$

### 4.3 Aerodynamic and radiation terms (Penman-Monteith)
1. Zero plane displacement height ($d_0$) and roughness lengths:
   $$d_0 = 0.667 \cdot h_{\text{crop}}, \quad z_{om} = 0.123 \cdot h_{\text{crop}}, \quad z_{oh} = 0.0123 \cdot h_{\text{crop}}$$
2. Aerodynamic resistance ($r_{aa}$):
   $$\text{AerDynRes} = \frac{\ln\left(\frac{z_w - d_0}{z_{om}}\right) \cdot \ln\left(\frac{z_t - d_0}{z_{oh}}\right)}{0.41^2}, \quad r_{aa} = \frac{\text{AerDynRes}}{W_{\text{ms}}}$$
3. Atmospheric pressure ($P_{\text{atm}}$) and latent heat of vaporization ($\lambda$):
   $$P_{\text{atm}} = 101.3 \cdot \left(\frac{293.0 - 0.0065 \cdot z_{\text{alt}}}{293.0}\right)^{5.253}$$
   $$\lambda = 2.501 - 0.002361 \cdot T_{\text{avg}}, \quad T_{\text{avg}} = \frac{T_{\max} + T_{\min}}{2}$$
4. Psychrometric constant ($\gamma$) and canopy-modified $\gamma^*$:
   $$\gamma = 0.0016286 \cdot \frac{P_{\text{atm}}}{\lambda}, \quad \gamma^* = \gamma \cdot \left(1.0 + \frac{r_c}{r_{aa}}\right)$$
5. Slope of saturation vapor pressure curve ($\Delta$):
   $$\Delta = \frac{2049.0 \cdot e_a(T_{\max})}{(T_{\max} + 237.3)^2} + \frac{2049.0 \cdot e_a(T_{\min})}{(T_{\min} + 237.3)^2}$$
6. Aerodynamic term ($\text{AeroTerm}$) and radiation term ($\text{RadTerm}$):
   $$\text{AeroTerm} = \frac{\gamma}{\Delta + \gamma^*} \cdot \frac{0.622 \cdot 3.486 \cdot 86400.0}{\text{AerDynRes} \cdot 1.01 \cdot (T_{\text{avg}} + 273.0)} \cdot W_{\text{ms}} \cdot (\bar{e}_a - e_a^{\text{dew}})$$
   $$\text{RadTerm} = \frac{\Delta}{\Delta + \gamma^*} \cdot \frac{R_n}{\lambda}$$
7. Potential combination evapotranspiration ($ET_{\text{comb}}$):
   $$ET_{\text{comb}} = \frac{\max(0.0, \text{RadTerm} + \text{AeroTerm})}{\rho_w(T_{\text{avg}})} \cdot 1000.0$$
   $$\rho_w(T) = \left(1.0 - 7.37 \times 10^{-6} \cdot (T - 4.0)^2 + 3.79 \times 10^{-8} \cdot (T - 4.0)^3\right) \cdot 1000.0$$

---

## 5. Atmospheric water switching with vapor coupling

### 5.1 Mode definitions and active-set switching
In the Rust architecture (`crates/hydrus-core/src/atmospheric.rs:10`):
- `UpperBoundaryMode::AtmosphericFlux`: Imposes net atmospheric flux demand $q_{\text{top}} = P - E_p$ as a Neumann condition at the surface.
- `UpperBoundaryMode::PondedHead`: Constrains the surface node to the critical pressure head (Dirichlet boundary) once surface supply or drying limits are reached.

Under vapor coupling (`lVapor=true`):
1. **Flux Mode (`AtmosphericFlux`)**:
   - The effective water flux applied to the surface node is $q_{\text{top}} = P - E_p$ (signed: positive into domain).
   - In `heat/vapor/mod.rs`, this corresponds to an `UpperBoundary::Flux { flux: q_top, ... }`.
   - The vapor-coupled solver incorporates vapor enhancement, latent heat flux, and boundary storage adjustments into the top node equation as established in `M6_VAPOR_CONTRACT.md` and `M6_VAPOR_CLOSED_PAIR_ROW_DESIGN.md`.
2. **Ponded/Constrained Mode (`PondedHead`)**:
   - When head reaches the ponding threshold ($h_{\text{critS}}$) or drops to the critical evaporative head ($h_{\text{critA}}$), the mode switches to `PondedHead`.
   - In `heat/vapor/mod.rs`, this corresponds to an `UpperBoundary::Head { head: h_crit, ... }`.
   - The surface Dirichlet row is assembled, and the resulting measured liquid plus vapor exchange flux is recorded for column mass balance.

### 5.2 Transition logic and stability guards
- Switching occurs between nonlinear Picard trials. If trial head $h_{\text{new}}(N) > h_{\text{critS}}$, the trial is rejected and retried in `PondedHead` mode with $h_{\text{top}} = h_{\text{critS}}$.
- If trial flux in `PondedHead` mode reverses sign such that infiltration demand is satisfied or evaporation rate drops below atmospheric demand, the boundary switches back to `AtmosphericFlux`.
- In all trials, the coupled two-pass loop (Water $\to$ Velocity $\to$ Heat $\to$ Water $\to$ Velocity $\to$ Heat) remains atomic: candidate states and boundary mode changes are committed only upon Picard and thermal convergence.
- Water-only simulations (`lVapor=false`) must remain bitwise identical to baseline behavior; existing atmospheric unit and acceptance suites must continue passing without modification.

---

## 6. Acceptance criteria and verification protocol

### 6.1 Parity against pinned Fortran reference (`vapor_met_parity.rs`)
1. Case execution: Run `reference/vapor_fortran/cases/vapor_met_*` with `lMeteo=true` and `lVapor=true` using the pinned Fortran binary (`tools/run_vapor_reference.py`).
2. Fixture generation: Extract `NOD_INF.OUT`, `A_LEVEL.OUT`, and `T_LEVEL.OUT` into `reference/vapor_fortran/fixtures/`.
3. Rust simulation: Execute the equivalent Rust scenario via `vapor_met_parity.rs`.
4. Tolerance comparison:
   - Pressure head ($h$): Max absolute difference within agreed tolerance (accounting for single-precision `REAL*4` in Fortran reference).
   - Liquid water content ($\theta$): Max difference within tolerance.
   - Temperature ($T$): Max difference within tolerance.
   - Trajectory fluxes ($v_{\text{top}}, v_{\text{bot}}$): Documented and verified.

### 6.2 Water and energy balance verification (`vapor_met_balance.rs`)
The total mass balance over the atmospheric time interval must close:
$$\Delta S_{\text{liquid}} + \Delta S_{\text{vapor}} = \int_{t_{\text{start}}}^{t_{\text{end}}} (q_{\text{top}} - q_{\text{bot}}) \, dt$$
where:
- $\Delta S_{\text{liquid}} = \sum_{i=1}^N \Delta x_i \cdot (\theta_{\text{new}, i} - \theta_{\text{old}, i})$
- $\Delta S_{\text{vapor}} = \sum_{i=1}^N \Delta x_i \cdot (\theta_{v, \text{new}, i} - \theta_{v, \text{old}, i})$
- $q_{\text{top}}$ is the accepted net surface exchange (accounting for rain, evaporation, and ponded storage).
- Mass balance discrepancy must remain below $1.0 \times 10^{-4}\,\text{cm}$ over the full trajectory.

---

## 7. Named rejections with triggers

The system must detect and reject unsupported, malformed, or out-of-scope configurations prior to state mutation, raising explicit typed errors:

| Rejection Name | Source Ref & Trigger Condition | Fortran Error Code | Typed Rust Error Variant |
|---|---|---|---|
| `CropHeightExceedsMeasurementHeight` | $0.667 \cdot h_{\text{crop}} \ge z_w \lor 0.667 \cdot h_{\text{crop}} \ge z_t$ (`TIME.FOR:639`) | Error 33 (`HYDRUS.FOR:830`) | `MeteoError::CropHeightExceedsMeasurementHeight { crop_height, wind_height, temp_height }` |
| `MeteoDeckMissingOrMalformed` | File `Meteo.in` missing, truncated, or format error (`INPUT.FOR:1043`) | Error 32 (`HYDRUS.FOR:829`) | `MeteoError::InvalidMeteoDeck { path, reason }` |
| `EnergyBalanceDeferred` | `lEnBal=true` requested with vapor/meteo (`INPUT.FOR:966`) | Deferred to G20 | `MeteoError::SurfaceEnergyBalanceUnsupported` |
| `InvertedTemperatureRange` | $T_{\min} > T_{\max}$ in meteorological record | Invalid input | `MeteoError::InvalidTemperatureRange { t_min, t_max }` |
| `RelativeHumidityOutOfRange` | $RH < 0.0$ or $RH > 100.0$ (%) | Physical domain | `MeteoError::RelativeHumidityOutOfRange { rh }` |
| `NegativeWindSpeed` | $W < 0.0$ in meteorological record | Physical domain | `MeteoError::NegativeWindSpeed { wind }` |
| `NegativeRadiation` | $R_{\text{ad}} < 0.0$ in meteorological record | Physical domain | `MeteoError::NegativeRadiation { radiation }` |
| `UnsupportedRadiationMode` | `iRadiation` outside supported set ($\{0, 1, 2\}$) | Unhandled mode | `MeteoError::UnsupportedRadiationMode { mode }` |
| `GrowthTableOverflow` | `iGrowth > 1000` (`INPUT.FOR:999-1003`) | Program STOP | `MeteoError::GrowthTableOverflow { count }` |
| `UnsupportedVaporLowerPair` | Atmospheric upper paired with seepage, drainage, or `qGWLF` | Architecture constraint | `VaporError::UnsupportedLowerBoundary(lower)` |

---

## 8. Open questions for GATE A approval

The following open questions are explicitly submitted for orchestrator and user review during GATE A:

1. **Bare soil vs. vegetated initial fixture**: In the first met fixture (`31.2.1`), should we use un-vegetated bare soil (`iCrop=0`, where $SCF=0, r_c=0, E_p = ET_{\text{comb}}$) to isolate evaporation and vapor coupling, or include grass cover (`iCrop=1`)?
2. **Sub-daily vs. daily meteorological stepping**: Fortran supports both daily aggregated stepping (`lMetDaily=false`) with sinusoidal diurnal variation, and sub-daily records (`lMetDaily=true`). Does wave 2 require both formats or should the initial fixture standardize on daily records?
3. **Critical evaporative head ($h_{\text{critA}}$) in vapor regime**: In pure liquid water, $h_{\text{critA}}$ stops liquid evaporation at the surface when soil dries. With vapor active, does $h_{\text{critA}}$ truncate total surface flux or does vapor continue to desorb across the dry boundary layer per source code?
4. **Boundary pair classification enum**: Should `VaporBoundaryPair` in `crates/hydrus-core/src/heat/vapor/mod.rs` be expanded to include explicit `Atmospheric` variants, or should `advance_atmospheric_step` dynamically decompose each step into existing `Flux` and `Head` variants before calling the vapor driver?
