# M6 Vapor Reference Deck Design: Dry Isothermal Closed Pair and Free-Drainage Pair (Task 30.3.2)

Status: **Approved Design Specification**. Documentation only; no deck files or fixtures are created in the repository in this task, and no reference runs are executed. Feeds Gate G30.3 and Gate G30.4 before any run in Wave 4.

Author: Antigravity Subagent (Task 30.3.2)  
Date: 2026-11-14  
Governing Plans: [EXECUTION_PLAN_30.md](../EXECUTION_PLAN_30.md), [STATE_30.md](../STATE_30.md), [GOALS.md](../GOALS.md) (G22.2-C3, G22.2-C6)  
Predecessor Documents: [M6_VAPOR_CLOSED_PAIR_FIXTURE_DESIGN.md](M6_VAPOR_CLOSED_PAIR_FIXTURE_DESIGN.md), [M6_VAPOR_FREE_DRAINAGE_ASSESSMENT.md](M6_VAPOR_FREE_DRAINAGE_ASSESSMENT.md), `planning/tasks/29.4.1.report.md`

---

## 1. Overview and Scope Boundaries

This document defines the formal design of two reference case pairs for the HYDRUS-1D Fortran reference runner (`hydrus-reference` built from pinned `src_reference/`):

1. **Deck A (Dry Isothermal Closed Pair):** A zero-flux closed column at extreme initial dryness ($h = -10000.0\text{ cm}$) under a completely uniform temperature field ($T = 25.0^\circ\text{C}$). Designed to resolve item **G22.2-C3** by isolating dry-regime vapor behavior in the absence of thermal driving forces.
2. **Deck B (Free-Drainage Vapor-On and Vapor-Off Pair):** A column with upper zero flux and lower unit-gradient free drainage, designed to provide reference benchmark evidence for item **G22.2-C6** under user-approved **Option (a)** (source parity matching `src_reference/WATFLOW.FOR:213`).

### Scope Constraints
- **Docs Only:** No deck directory is created under `reference/vapor_fortran/cases/` in this task.
- **No Reference Execution:** No reference binary is run during this task. Reference execution occurs strictly in Wave 4 (`.development/30.4.1/` and `.development/30.4.2/`) following explicit user approval at Gate G30.3 and Gate G30.4.
- **Repository Immutability:** No file under `reference/` is altered. Existing fixtures in `reference/vapor_fortran/fixtures/` and criteria in `reference/vapor_fortran/criteria.json` remain untouched.

---

## 2. Deck A: Dry Isothermal Closed Pair

### 2.1 Rationale and Lineage
In Orchestration 29 (Task 29.4.1), attempts to simulate a closed zero-flux column (`KodTop=-1`, `KodBot=-1`, `rTop=rBot=0.0`) at initial dry head $h = -10000.0\text{ cm}$ with an imposed 40-degree thermal gradient ($45^\circ\text{C}$ top to $5^\circ\text{C}$ bottom) suffered numerical divergence (`NaN` generation and time-step collapse down to $dt_{\min}$).

As diagnosed in `planning/tasks/29.4.1.report.md`, with dry soil moisture capacity $C(h) = d\theta/dh \approx 4 \times 10^{-7}\text{ cm}^{-1}$ at $h = -10000\text{ cm}$, the thermal vapor flux ($v_{\text{top}} \sim 10^{-5}\text{ cm/h}$) driven by the $1.0^\circ\text{C/cm}$ gradient induced massive, unresolvable head excursions in the free top boundary node ($|h| > 10^5\text{ cm}$ within 0.5 h). Decreasing the gradient by a factor of 4 (Candidate C, $25^\circ\text{C} \to 15^\circ\text{C}$) delayed blow-up from $t = 1.7\text{ h}$ to $t = 72\text{ h}$, confirming that the thermal vapor flux was the primary driver of numerical instability. Candidate D stabilized the system by wetting the column to $h = -100\text{ cm}$, which became the committed wet closed pair, but left open the dry regime.

User Decision **D3** (`EXECUTION_PLAN_30.md`) selected the **isothermal dry closed deck** strategy: by eliminating the thermal gradient entirely ($\partial T / \partial z = 0$), thermal vapor flux is eliminated ($q_{vT} = -K_{vT} \partial T / \partial z = 0$), allowing evaluation of whether the solver converges in the dry regime under closed Neumann boundaries.

### 2.2 Base Deck Lineage
- **Base Case:** `reference/vapor_fortran/cases/vapor_closed_flux_on/` (vapor on) and `reference/vapor_fortran/cases/vapor_closed_flux_off/` (vapor off).
- **Proposed Case Identifiers:**
  - `vapor_dry_isothermal_on` (vapor on, $l\text{Vapor} = \text{t}$)
  - `vapor_dry_isothermal_off` (vapor off control, $l\text{Vapor} = \text{f}$)

### 2.3 Precise Line-by-Line Changes for Deck A

#### 2.3.1 `SELECTOR.IN` Diffs
Starting from `reference/vapor_fortran/cases/vapor_closed_flux_on/SELECTOR.IN` (46 lines):

| Line | Original Content in `vapor_closed_flux_on` | Modified Content in `vapor_dry_isothermal_on` | Explanation |
|:---:|:---|:---|:---|
| **3** | `Heat-only derived deck for source-matched fixture production...` | `Dry isothermal closed pair for source-matched fixture production.` | Informational header description (Block A). |
| **12** | `f f f t f f` (vapor on) / `f f f f f f` (vapor off) | `f f f t f f` (vapor on) / `f f f f f f` (vapor off) | `lVapor` toggle between on and off companion cases. |
| **19** | `f f -1 f` | `f f -1 f` | **Unchanged:** `TopInf=f`, `WLayer=f`, `KodTop=-1`, `InitCond=f`. |
| **21** | `f f f f -1 f 0` | `f f f f -1 f 0` | **Unchanged:** `BotInf=f`, `qGWLF=f`, `FreeD=f`, `SeepF=f`, `KodBot=-1`, `DrainF=f`, `hSeep=0`. |
| **23** | `0.0 0.0 0.0` | `0.0 0.0 0.0` | **Unchanged:** `rTop=0.0`, `rBot=0.0`, `rRoot=0.0`. |
| **45** | `1 45 1 5` | `1 25 1 25` | **Changed:** Boundary temperature record. Top boundary $T_{\text{top}} = 25.0^\circ\text{C}$ (`kTopT=1`, Dirichlet); bottom boundary $T_{\text{bot}} = 25.0^\circ\text{C}$ (`kBotT=1`, Dirichlet). Replaces the $45^\circ\text{C} \to 5^\circ\text{C}$ gradient with uniform $25^\circ\text{C}$. |

All other lines (1–2, 4–11, 13–18, 20, 22, 24–44, 46) are completely identical to `vapor_closed_flux_on/SELECTOR.IN`.

#### 2.3.2 `PROFILE.DAT` Diffs
Starting from `reference/vapor_fortran/cases/vapor_closed_flux_on/PROFILE.DAT` (46 lines, 41 mesh nodes from $z = 0.0$ to $z = -40.0\text{ cm}$):

- **Header Records (Lines 1–4):** Completely unchanged (`Pcp_File_Version=4`, `1`, `heat-only derived profile`, `41 0 0`).
- **Nodal Records (Lines 5–45, Nodes 1 to 41):**
  - **Column 3 (Initial Pressure Head $h$):** Replaced with `-10000.000` on all 41 lines (was `-100.000`).
  - **Column 10 (Initial Temperature $T$):** Replaced with `25.00` on all 41 lines (was $45.00$ at node 1, decreasing linearly by $1.0^\circ\text{C}$ per node to $5.00$ at node 41).
  - **Columns 1, 2, 4–9:** Completely unchanged ($i = 1..41$, $x = 0.0..-40.0$, $\text{Mat}=1$, $\text{Lay}=1$, $\text{Beta}=0$, $\text{Axz}=1$, $\text{Bxz}=1$, $\text{Dxz}=1$).
- **Footer Record (Line 46):** Completely unchanged (`0`).

```
Example Profile Edit (Line 5, Node 1):
  Original:  1 -0.0 -100.000 1 1 0 1 1 1 45.00
  Modified:  1 -0.0 -10000.000 1 1 0 1 1 1 25.00

Example Profile Edit (Line 45, Node 41):
  Original: 41 -40.0 -100.000 1 1 0 1 1 1 5.00
  Modified: 41 -40.0 -10000.000 1 1 0 1 1 1 25.00
```

### 2.4 The One-Change Rule for Deck A
The one-change rule enforces scientific traceability across benchmark experiments:
- **Relative to the divergent dry attempt in Task 29.4.1 (which had $h = -10000\text{ cm}$ and $T = 45^\circ\text{C} \to 5^\circ\text{C}$):** The single modification is **the elimination of the thermal gradient** ($T$ set to uniform $25.0^\circ\text{C}$ across boundary conditions and profile nodes), setting thermal vapor flux $q_{vT} \equiv 0$.
- **Relative to the wet closed baseline (`vapor_closed_flux_on`):** Deck A constitutes the **dry isothermal counterpart**, holding the closed Neumann boundary conditions (`KodTop=-1`, `KodBot=-1`, `rTop=rBot=0.0`) and column geometry invariant while testing the $h = -10000\text{ cm}$ dry regime at the profile mean temperature $T = (45 + 5)/2 = 25.0^\circ\text{C}$.

### 2.5 Expected Failure Signatures
If the isothermal condition fails to stabilize the dry closed column in Task 30.4.1, the failure will exhibit one of the following concrete numerical signatures:
1. **Time-Step Collapse (`dt < dtMin`):** `WATFLOW.FOR:127-146` cuts the time step by `DMul2 = 0.5` after non-convergence (`ItCum > MaxIt = 20`) until $dt < dt_{\min} = 0.01\text{ h}$, triggering console error: `The rule that dt.ge.dtMin is violated`.
2. **Boundary Head Oscillation / Runaway:** Perturbations in isothermal vapor flux $q_{vh} = -K_{vh} \partial h / \partial z$ interacting with negligible moisture capacity ($C \approx 4 \times 10^{-7}\text{ cm}^{-1}$) cause $|h_{\text{top}}|$ or $|h_{\text{bot}}|$ to exceed $10^6\text{ cm}$ or turn positive within early steps ($t < 1.0\text{ h}$).
3. **Floating-Point NaN / Arithmetic Exception:** `NaN` appears in `T_LEVEL.OUT` or `NOD_INF.OUT`, caused by floating underflow/overflow in relative humidity $H_r = \exp(h M g / R T)$ or zero division in moisture capacity calculations.
4. **Truncated Horizon:** Simulation halts prematurely without reaching $t_{\max} = 240\text{ h}$, failing to output print records for times 6, 24, 72, or 240 h.

---

## 3. Deck B: Free-Drainage Vapor-On and Off Pair

### 3.1 Rationale and Lineage
Task 30.1.3 ([M6_VAPOR_FREE_DRAINAGE_ASSESSMENT.md](M6_VAPOR_FREE_DRAINAGE_ASSESSMENT.md)) revealed that `src_reference/WATFLOW.FOR:202-229` contains an internal algebraic coupling in subroutine `Reset`:
```fortran
211:       if(lVapor) ConB=ConB+(ConVh(1)+ConVh(2))/2.
212:       S(1)=-ConB/dxB
213:       if(FreeD) rBot=-ConB*Grav*fRE
```
At Gate **G30.2**, the user selected **Option (a)**: match `WATFLOW.FOR` as written ($r_{\text{Bot}} = -(K_L + K_{vh}) \cdot \text{Grav}$), log proposed defect SD-008 in project documentation, and admit `FreeDrainage` paired exclusively with upper `Flux` in `hyd.rs`.

Deck B establishes the official reference benchmark fixtures for free drainage with vapor active, against which the Rust implementation in Task 30.2.1 will be verified in Wave 5 (Task 30.5.1).

### 3.2 Base Deck Lineage
- **Base Case:** `reference/vapor_fortran/cases/vapor_closed_flux_on/` and `vapor_closed_flux_off/`.
- **Proposed Case Identifiers:**
  - `vapor_free_drainage_on` (vapor on, $l\text{Vapor} = \text{t}$)
  - `vapor_free_drainage_off` (vapor off control, $l\text{Vapor} = \text{f}$)

### 3.3 Precise Line-by-Line Changes for Deck B

#### 3.3.1 `SELECTOR.IN` Diffs
Starting from `reference/vapor_fortran/cases/vapor_closed_flux_on/SELECTOR.IN` (46 lines):

| Line | Original Content in `vapor_closed_flux_on` | Modified Content in `vapor_free_drainage_on` | Explanation |
|:---:|:---|:---|:---|
| **3** | `Heat-only derived deck for source-matched fixture production...` | `Free-drainage lower boundary with vapor for source-matched fixture production.` | Informational header description (Block A). |
| **12** | `f f f t f f` (vapor on) / `f f f f f f` (vapor off) | `f f f t f f` (vapor on) / `f f f f f f` (vapor off) | `lVapor` toggle between on and off companion cases. |
| **19** | `f f -1 f` | `f f -1 f` | **Unchanged:** `TopInf=f`, `WLayer=f`, `KodTop=-1`, `InitCond=f` (upper zero flux retained). |
| **20** | `BotInf qGWLF FreeD SeepF KodBot DrainF hSeep` | `BotInf qGWLF FreeD SeepF KodBot DrainF hSeep` | Block B bottom boundary variable labels. |
| **21** | `f f f f -1 f 0` | `f f t f -5 f 0` | **Changed:** Free-drainage lower boundary activation. Field 3 (`FreeD`) changes from `f` to `t`. Field 5 (`KodBot`) changes from `-1` to `-5`. |
| **22** | `rTop rBot rRoot` | `rTop rBot rRoot` | **Unchanged & Retained:** Header record for flux inputs. |
| **23** | `0.0 0.0 0.0` | `0.0 0.0 0.0` | **Unchanged & Retained:** Top flux $r_{\text{Top}} = 0.0\text{ cm/h}$. |

All other lines (1–2, 4–11, 13–18, 24–46) are completely identical to `vapor_closed_flux_on/SELECTOR.IN`.

#### 3.3.2 Retention of `SELECTOR.IN` Lines 22–23 (`rTop rBot rRoot`)
It is critical to note why lines 22–23 MUST remain in `SELECTOR.IN` for Deck B.
In `src_reference/INPUT.FOR:63-67`:
```fortran
63:       if((.not.TopInF.and.KodTop.eq.-1).or.
64:      !   (.not.BotInF.and.KodBot.eq.-1.and.
65:      !   .not.qGWLF.and..not.FreeD.and..not.SeepF.and..not.qDrain)) then
66:         read(30,*,err=902)
67:         read(30,*,err=902) rTop,rBot,rRoot
68:       else
69:         rTop=0.
70:         rBot=0.
71:         rRoot=0.
72:       end if
```
Because the top boundary is prescribed flux (`TopInF = .false.` and `KodTop = -1`), the first condition `(.not.TopInF.and.KodTop.eq.-1)` evaluates to `.true.` regardless of the bottom boundary settings. Therefore, `INPUT.FOR` explicitly executes `read(30,*,err=902)` for the header and values `rTop, rBot, rRoot`.
If lines 22–23 were deleted, `INPUT.FOR` would attempt to read lines 24–25 (`hTab1 hTabN`) as flux inputs, resulting in input failure (`ierr=2`). In execution, `rTop = 0.0` is used as the top boundary flux, while `rBot = 0.0` is read and immediately overwritten in `WATFLOW.FOR:213` by the dynamic free-drainage flux.

#### 3.3.3 `PROFILE.DAT` Diffs
`PROFILE.DAT` for Deck B is **completely identical** to `reference/vapor_fortran/cases/vapor_closed_flux_on/PROFILE.DAT`.
- 41 mesh nodes ($z = 0.0$ to $z = -40.0\text{ cm}$).
- Uniform initial head $h = -100.000\text{ cm}$ (Column 3).
- Linear thermal gradient $45.00^\circ\text{C} \to 5.00^\circ\text{C}$ (Column 10).
- Zero modifications across all 46 lines.

### 3.4 The One-Change Rule for Deck B
The single modification relative to `reference/vapor_fortran/cases/vapor_closed_flux_on/` is **the replacement of the zero-flux lower boundary with unit-gradient free drainage**:
$$\text{Line 21: } \texttt{f f f f -1 f 0} \longrightarrow \texttt{f f t f -5 f 0}$$
All other domain properties (upper boundary `KodTop=-1`, $r_{\text{Top}}=0.0$, initial head $h = -100\text{ cm}$, thermal gradient $45^\circ\text{C} \to 5^\circ\text{C}$, soil hydraulic and thermal material parameters, and mesh node coordinates) remain strictly identical.

### 3.5 Physical Coupling Analysis under Option (a)
Under Option (a), the bottom boundary drainage flux dynamically evaluated at each Picard iteration is:
$$r_{\text{Bot}} = - (K_{L,\text{face}} + K_{vh,\text{face}}) \cdot \text{Grav} \cdot \text{fRE}$$
In the bottom node RHS vector `PB` (`WATFLOW.FOR:221-222`), the gravity term $B = K_{L,\text{face}} \cdot \text{Grav}$ combines with $r_{\text{Bot}}$, leaving:
$$B + r_{\text{Bot}} = - K_{vh,\text{face}} \cdot \text{Grav} \cdot \text{fRE}$$
In the wet regime ($h = -100\text{ cm}$), liquid conductivity dominates ($K_L \approx 10^{-2}\text{ to } 10^{-4}\text{ cm/h}$ vs $K_{vh} \approx 10^{-7}\text{ cm/h}$), so the vapor gravity residual is small ($|K_{vh} \cdot g| \ll |K_L \cdot g|$). However, because `vapor_free_drainage_on` incorporates this term while `vapor_free_drainage_off` excludes vapor entirely ($K_{vh} \equiv 0$), this pair provides unambiguous discriminating evidence of vapor-gravity coupling at the lower boundary.

---

## 4. Expected Outputs and Target Print Horizon

Both decks will be executed to $t_{\max} = 240.0\text{ hours}$ using the standard print time schedule defined in Block C (`MPL=4`, `TPrint = 6, 24, 72, 240 hours`).

### 4.1 Expected Output Files
Upon successful execution, each run directory in `.development/` will contain:

1. `T_LEVEL.OUT`: Time-step evolution and global boundary flux summary:
   - Simulation time $t$, time-step size $dt$, Picard iteration count per step.
   - Surface and bottom boundary fluxes: `vTop`, `vBot`.
   - Surface and bottom boundary pressure heads: `hTop`, `hBot`.
   - Cumulative infiltration and drainage: `CumQ`, `CumExt`.
   - Subregion water and heat storage volumes: `Volume`, `HeatVol`.
2. `NOD_INF.OUT`: Detailed 41-node profiles printed at $t = 0, 6, 24, 72, 240\text{ hours}$:
   - Node coordinate $z$ (cm).
   - Pressure head $h$ (cm).
   - Liquid moisture content $\theta$ ($\text{cm}^3/\text{cm}^3$).
   - Temperature $T$ ($^\circ\text{C}$).
   - Hydraulic conductivity $K$ (cm/h).
   - Moisture capacity $C = d\theta/dh$ ($1/\text{cm}$).
   - Liquid Darcian flux $v$ (cm/h).
3. `NOD_INF_V.OUT` (produced only when $l\text{Vapor} = \text{t}$):
   - Total isothermal conductivity $K = K_L + K_{vh}$ (cm/h).
   - Total thermal conductivity $K_T = K_{LT} + K_{vT}$ ($\text{cm}^2/\text{h}^\circ\text{C}$).
   - Isothermal vapor conductivity $K_{vh}$ (cm/h).
   - Thermal vapor conductivity $K_{vT}$ ($\text{cm}^2/\text{h}^\circ\text{C}$).
   - Velocity components: liquid flux $v_L$, vapor flux $v_v$, total flux $v_{\text{total}}$, isothermal vapor flux $v_{v,\text{iso}}$, and thermal vapor flux $v_{v,\text{term}}$ (cm/h).
   *(Note: For Deck A, $v_{v,\text{term}} \equiv 0.0$ at all nodes and times due to uniform $T = 25^\circ\text{C}$.)*
4. `BALANCE.OUT` (supported via runner update in Task 30.3.1):
   - Mass balance table reporting total profile water volume, instantaneous boundary flux rates, and relative mass balance error per print interval.

---

## 5. Run Budget and Escalation 5 Stop Rules

Execution of reference cases involves non-linear numerical solvers running compiled GNU Fortran binaries. To prevent infinite loops, disk saturation, and untracked exploratory knob-tuning, strict operational limits are established per **Escalation 5** of `EXECUTION_PLAN_30.md`.

### 5.1 Run Budget
- **Permitted Run Count:** Exactly **one (1)** execution per approved deck configuration:
  - Task 30.4.1: Exactly 1 run for `vapor_dry_isothermal_on`. (If converged, exactly 1 run for companion control `vapor_dry_isothermal_off`).
  - Task 30.4.2: Exactly 1 run for `vapor_free_drainage_on`; exactly 1 run for `vapor_free_drainage_off`.
- **Sandbox Working Directories:**
  - Task 30.4.1: `.development/30.4.1/`
  - Task 30.4.2: `.development/30.4.2/`
  *(No files written to `reference/vapor_fortran/` or `src_reference/`).*
- **Execution Binary:** Reused reference executable at `.development/29.3.3/reference-build/hydrus-reference` (SHA-256 verified at Wave 0 baseline).
- **Wall-Clock Timeout:** Hard limit of **600 seconds (10 minutes)** per run. The runner process must be spawned under timeout supervision.
- **Maximum Output Disk Budget:** Hard ceiling of **50 MB** total disk usage per run directory. If total output exceeds 50 MB, the process must be terminated immediately (preventing runaways such as the 1.5 GB output observed in 29.4.1 run B).
- **Time Horizon Controls:** $t_{\text{init}} = 0\text{ h}$, $t_{\max} = 240\text{ h}$, $dt = 0.1\text{ h}$, $dt_{\min} = 0.01\text{ h}$, $dt_{\max} = 0.5\text{ h}$.

### 5.2 Stop Rules (Escalation 5 Protocol)
Any of the following conditions constitutes an immediate failure requiring the executing worker to **STOP IMMEDIATELY**:

1. **Rule 1: Time-Step Floor Breach (`dt < dtMin`)**  
   If the solver outputs `The rule that dt.ge.dtMin is violated` or exits with a non-zero exit code due to convergence failure, terminate execution.
2. **Rule 2: Numerical Non-Finite Values (`NaN` / `Inf`)**  
   If `NaN`, `NaN%`, `Infinity`, or `-NaN` appears anywhere in `T_LEVEL.OUT`, `NOD_INF.OUT`, `NOD_INF_V.OUT`, stdout, or stderr, terminate execution immediately.
3. **Rule 3: Disk / Output Runaway**  
   If the aggregate size of output files in the run directory exceeds 50 MB or internal time step count exceeds 50,000 steps without reaching $t = 240\text{ h}$, send `SIGTERM` followed by `SIGKILL`.
4. **Rule 4: Incomplete Horizon / Missing Print Records**  
   If the simulation halts or aborts prior to printing complete nodal blocks at all designated print times ($t = 6, 24, 72, 240\text{ h}$), mark as non-converged.
5. **Rule 5: Strict Zero-Retries Policy (No Knob-Tuning)**  
   If a run fails to converge, the worker **MUST NOT** alter deck parameters (such as $dt_{\min}$, $DMul$, $MaxIt$, tolerances $\text{TolH}/\text{TolTh}$, or mesh spacing) and re-run. The worker must capture the exact failure record (exit code, non-convergence message count, NaN count, last printed time, boundary head trajectory) in `planning/tasks/30.4.x.report.md` and report to the orchestrator. Retrying with alternative knobs requires a new user gate.

---

## 6. Pre-Run Gate Checklist (Feeds G30.3 & G30.4)

Before any execution occurs in Wave 4, the following gating conditions must be verified:

- [x] Deck A line-by-line diffs specified with explicit line numbers for `SELECTOR.IN` and `PROFILE.DAT`.
- [x] Deck A one-change rule and failure signatures explicitly defined.
- [x] Deck B line-by-line diffs specified with explicit line numbers for `SELECTOR.IN` Block B (lines 20–21).
- [x] Justification for retention of lines 22–23 (`rTop rBot rRoot`) established from `INPUT.FOR:63-67`.
- [x] Deck B one-change rule and physical coupling analysis under Option (a) established.
- [x] Expected output files, print horizons, and discrimination fields defined.
- [x] Run budget (1 run per variant, 600 s timeout, 50 MB cap) and Escalation 5 stop rules formulated.
- [x] Zero files modified under `reference/` or `src_reference/`.

*Proceed to Gate G30.3 approval.*
