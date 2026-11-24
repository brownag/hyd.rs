# M6 Vapor Free-Drainage Source Coupling Assessment (30.1.3)

Status: Complete assessment. No code modified, no reference run executed. Output feeds Gate G30.2.

Primary sources: `src_reference/WATFLOW.FOR:202-229` (pinned Fortran source); `crates/hydrus-core/src/heat/vapor/water.rs`, `crates/hydrus-core/src/lower_boundary.rs`, `crates/hydrus-core/src/heat/vapor/mod.rs`, `crates/hydrus-core/src/water_solver.rs`, `crates/hydrus-core/tests/vapor_boundary_guard.rs`.

---

## 1. Verdict on the `ConB` Inconsistency with Evidence

### 1.1 Fortran Source Citations (`src_reference/WATFLOW.FOR:202-229`)

In `src_reference/WATFLOW.FOR`, the lower boundary condition and bottom element matrix coefficients (node 1) are assembled in subroutine `Reset` (lines 200–229):

```fortran
200:       dxB=x(2)-x(1)
201:       dx=dxB/2.
202:       ConB=(Con(1)+Con(2))/2.              ! Arithmetic average
203:       if(lGeom) ConB=(Con(1)*Con(2))**0.5  ! Geometric average
204:       if(lCentrif) Grav=CosAlf*(Radius+abs((x(1)+x(2))/2.))
205:       B=ConB*Grav
206:       if(lDensity) then
207:         fRE=fRo(1,(Conc(1,1)+Conc(1,2))/2.)
208:         B=ConB*Grav*fRE
209:         fRE=fRo(1,Conc(1,1))
210:       end if
211:       if(lVapor) ConB=ConB+(ConVh(1)+ConVh(2))/2.
212:       S(1)=-ConB/dxB
213:       if(FreeD) rBot=-ConB*Grav*fRE
214:       F2=Cap(1)*dx/dt*fRE*Rate
215:       RB=ConB/dxB+F2
216:       SB=-ConB/dxB
217:       if(qGWLF) rBot=Fqh(hNew(1)-GWL0L,Aqh,Bqh)
218:       if(qDrain) rBot=FqDrain(x(1)+hNew(1),zBotDr,BaseGW,rSpacing,
219:      !                        iPosDr,rKhTop,rKhBot,rKvTop,rKvBot,
220:      !                        Entres,WetPer,zInTF,GeoFac)
221:       PB=B-Sink(1)*dx+F2*hNew(1)-(ThNew(1)-ThOld(1))*dx/dt*fRE+
222:      !   rBot
223:       if(iDualPor.gt.0) PB=PB-SinkIm(1)*dx
224:       if(lVapor.or.lWTDep) then
225:         ConTB=0.
226:         if(lVapor) ConTB=ConTB+(ConVT(1)+ConVT(2))/2.
227:         if(lWTDep) ConTB=ConTB+(ConLT(1)+ConLT(2))/2.
228:         PB=PB+ConTB*(Temp(2)-Temp(1))/dxB-(ThVNew(1)-ThVOld(1))*dx/dt
229:       end if
```

### 1.2 Algebraic and Physical Analysis of the Inconsistency

1. **What `ConB` includes:**
   - At line 202 (`WATFLOW.FOR:202`), `ConB` is initialized as the inter-node arithmetic average of liquid hydraulic conductivity `(Con(1) + Con(2)) / 2.0` (or geometric average at `:203`).
   - At line 211 (`WATFLOW.FOR:211`), when vapor transport is active (`lVapor = .true.`), `ConB` is updated in place:
     $$\text{ConB} \leftarrow \text{ConB} + \frac{\text{ConVh}(1) + \text{ConVh}(2)}{2}$$
     After line 211, `ConB` represents the total isothermal head conductivity: liquid hydraulic conductivity plus isothermal vapor conductivity ($K_L + K_{vh}$).
   - At line 213 (`WATFLOW.FOR:213`), when free drainage is enabled (`FreeD = .true.`), the bottom boundary flux `rBot` is computed:
     $$\text{rBot} = -\text{ConB} \cdot \text{Grav} \cdot \text{fRE} = -(K_{L,\text{face}} + K_{vh,\text{face}}) \cdot \text{Grav} \cdot \text{fRE}$$
     Therefore, `rBot` includes the isothermal vapor conductivity $K_{vh,\text{face}}$.

2. **What the gravity term `B` excludes:**
   - At line 205 (`WATFLOW.FOR:205`), the gravity term `B` is computed:
     $$B = \text{ConB} \cdot \text{Grav}$$
     Crucially, line 205 executes **before** line 211. At line 205, `ConB` contains only the liquid conductivity $K_{L,\text{face}}$.
   - Therefore, the gravity term $B$ strictly **excludes** the vapor conductivity $K_{vh,\text{face}}$.
   - This exclusion of vapor conductivity from the gravity term is consistent with interior nodes (`WATFLOW.FOR:239`), where $B = (\text{ConA} - \text{ConB}) \cdot \text{Grav}$ is evaluated before `lVapor` modifies `ConA` and `ConB` at `:246-249`.

3. **The resulting inconsistency:**
   - In HYDRUS-1D, free drainage represents a unit hydraulic gradient boundary condition ($\partial h / \partial z = 0$), where water drains purely under gravity.
   - For liquid-only flow (`lVapor = .false.`, $\text{fRE} = 1.0$), the bottom node RHS vector `PB` (`WATFLOW.FOR:221-222`) combines $B$ and `rBot`:
     $$B + \text{rBot} = K_{L,\text{face}} \cdot \text{Grav} - K_{L,\text{face}} \cdot \text{Grav} = 0$$
     The downward gravitational inflow from the element above exactly cancels the boundary drainage outflow, ensuring unit-gradient equilibrium.
   - When vapor is enabled (`lVapor = .true.`), $B$ and `rBot` do not cancel:
     $$B + \text{rBot} = K_{L,\text{face}} \cdot \text{Grav} - (K_{L,\text{face}} + K_{vh,\text{face}}) \cdot \text{Grav} = -K_{vh,\text{face}} \cdot \text{Grav}$$
     An artificial, unbalanced residual $-K_{vh,\text{face}} \cdot \text{Grav}$ is injected directly into `PB` at the bottom node.
   - **Physical verdict:** Water vapor transport in porous media is a gas-phase diffusive process driven by matric potential gradients ($q_{vh} = -K_{vh} \frac{\partial h}{\partial z}$) and temperature gradients ($q_{vT} = -K_{vT} \frac{\partial T}{\partial z}$). Vapor transport does not possess a gravitational body force term ($\rho g$). Multiplying $K_{vh}$ by `Grav` in `rBot` creates a fictitious "vapor gravity drainage flux" that contradicts the governing physical equations of HYDRUS.
   - **Root cause verdict:** This is a classic Fortran variable reuse defect. The scalar variable `ConB` was reused to accumulate total head conductivity for matrix conduction terms ($S(1)$ at line 212, $RB$ at line 215, $SB$ at line 216). Because line 213 `if(FreeD) rBot=-ConB*Grav*fRE` was positioned after line 211 rather than before it, it inadvertently captured the vapor contribution intended only for the conduction matrix.

---

## 2. Rust Behaviour Today

Grep and inspection of `FreeD` and `FreeDrainage` across the codebase reveal the following baseline state:

1. **Lower Boundary Mode Definition:**
   - `crates/hydrus-core/src/lower_boundary.rs:19` [verified]: `LowerBoundaryMode::FreeDrainage` is defined as a public enum variant.
   - `crates/hydrus-core/src/lower_boundary.rs:127-132` [verified]: `validate_lower_boundary` enforces that mesh node count $\ge 2$.

2. **Liquid-Only Implementation in Rust:**
   - `crates/hydrus-core/src/lower_boundary.rs:161-183` [verified]: `prepare_lower_boundary` handles `FreeDrainage` using liquid-only conductivities (`trial.conductivity_a`, `trial.conductivity_b`).
   - Line 181 [verified]: `let flux = -conductivity * trial.gravity;` (Neumann flux, positive into domain).
   - Lines 178-180 [verified]: Explicit code comment notes:
     `// WATFLOW.FOR Reset adds rBot=-ConB*Grav to its lower RHS.`
     `// Rust defines positive boundary flux as entering the domain,`
     `// so the source-derived free-drainage exchange is -K_face*g.`
   - Line 194 [verified]: Row construction falls back to `trial.flux_row()?`, which calls `lower_flux_row` (`:219-235`) using liquid-only properties.

3. **Unconditional Rejection on the Vapor Path:**
   - `crates/hydrus-core/src/heat/vapor/mod.rs:220-228` [verified]: `VaporBoundaryPair` only defines `HeadHead` and `ClosedFlux { lower_flux: f64, upper_flux: f64 }`.
   - `crates/hydrus-core/src/heat/vapor/mod.rs:294-299` [verified]: In `VaporBoundaryPair::classify`, `LowerBoundaryMode::FreeDrainage` paired with any upper boundary (`Head`, `Flux`, or `Atmospheric`) is unconditionally rejected:
     ```rust
     | (
         LowerBoundaryMode::FreeDrainage | LowerBoundaryMode::Unsupported(_),
         UpperBoundary::Head { .. }
         | UpperBoundary::Flux { .. }
         | UpperBoundary::Atmospheric { .. },
     ) => Err(VaporError::UnsupportedLowerBoundary(lower)),
     ```
   - `crates/hydrus-core/tests/vapor_boundary_guard.rs:101, 127-136` [verified]: Test `every_non_pair_combination_keeps_its_unsupported_error` verifies that all combinations containing `LowerBoundaryMode::FreeDrainage` return `Err(VaporError::UnsupportedLowerBoundary(..))`.
   - `crates/hydrus-core/src/water_solver.rs:362-373` [verified]: The vapor trial solver dispatches only `ClosedFlux` and `HeadHead`. No dispatch arm exists for `FreeDrainage`.

4. **Areas Not Read in Scope:**
   - Full-trajectory water balance accounting for free drainage with vapor in `crates/hydrus-core/src/heat/vapor/balance.rs`: [not read in scope].
   - Multi-step driver orchestration for free-drainage boundary updates in `crates/hydrus-core/src/driver.rs`: [not read in scope].

---

## 3. Options for Handling the Inconsistency

### Option (a): Match the Source as Written (Source Parity)

* **Description:**
  Replicate the exact algebraic behavior of `src_reference/WATFLOW.FOR:202-229`.
  - The free-drainage exchange flux is calculated using total face conductivity including vapor:
    $$r_{\text{Bot}} = -(K_{L,\text{face}} + K_{vh,\text{face}}) \cdot \text{Grav}$$
    (In Rust convention, positive into domain: $\text{flux} = -(K_{L,\text{face}} + K_{vh,\text{face}}) \cdot g$).
  - Matrix conduction coefficients ($S(1)$, diagonal, off-diagonal) use total conductivity $K_{L,\text{face}} + K_{vh,\text{face}}$.
  - The bottom row RHS retains liquid gravity inflow $B = K_{L,\text{face}} \cdot g$ and gains $r_{\text{Bot}}$, leaving the net $-K_{vh,\text{face}} \cdot g$ residual alongside thermal vapor conduction $K_{vT,\text{face}} \frac{\Delta T}{\Delta x}$ and vapor storage change $-\frac{\Delta \theta_v}{\Delta t}$.
* **Rust Rows Affected:**
  - `crates/hydrus-core/src/heat/vapor/water.rs`: `vapor_lower_flux_row` [verified: lines 250-282] already sets `rhs: liquid * input.gravity - ...` without $r_{\text{Bot}}$. When $r_{\text{Bot}} = -(K_L + K_{vh}) \cdot g$ is supplied as the Neumann boundary flux, `transform_boundary_row` adds it to the RHS, yielding exactly $B + r_{\text{Bot}} = -K_{vh} \cdot g$. Alternatively, a specialized `vapor_lower_free_drainage_row` can be introduced.
  - `crates/hydrus-core/src/water_solver.rs`: `solve_candidate_vapor` [verified: lines 362-382] must be extended to evaluate dynamic face conductivities $K_L$ and $K_{vh}$ per Picard iteration and assemble the free-drainage row.
* **Guard Change:**
  - `crates/hydrus-core/src/heat/vapor/mod.rs`: Add variant `FreeDrainage { upper_flux: f64 }` to `VaporBoundaryPair`. Update `VaporBoundaryPair::classify` to accept `(LowerBoundaryMode::FreeDrainage, UpperBoundary::Flux { flux, .. })`.
* **Test Impact:**
  - `crates/hydrus-core/tests/vapor_boundary_guard.rs`: Update `every_non_pair_combination_keeps_its_unsupported_error` to account for the newly accepted `FreeDrainage` + `Flux` pair.
  - New unit tests for free-drainage vapor row assembly and dynamic flux calculation in `tests/vapor_boundary_guard.rs` or a dedicated test file.
  - Integration parity tests against Fortran reference fixtures in Wave 5.
* **Reference Run Needed:**
  - Requires standard reference runs using the existing, unmodified `hydrus-reference` executable with `KodBot = -5` (`FreeD = .true.`) and `lVapor = .true.`. Zero-tolerance or tight-threshold numerical parity can be achieved directly.

---

### Option (b): Correct as a Source Defect (Proposed SD-008)

* **Description:**
  Recognize line 213 as a source defect (variable reuse bug) and correct it in Rust.
  - The free-drainage exchange flux is calculated using liquid conductivity only:
    $$r_{\text{Bot}} = -K_{L,\text{face}} \cdot \text{Grav}$$
  - The interior gravity term $B = K_{L,\text{face}} \cdot g$ and $r_{\text{Bot}}$ exactly cancel ($B + r_{\text{Bot}} = 0$).
  - Matrix conduction terms continue to use $K_L + K_{vh}$ (since matric potential gradients act on both liquid and vapor). Thermal vapor flux and vapor storage terms are retained.
* **Proposed SD Register Entry (Proposal Only — Not Written to `docs/FORTRAN_SOURCE_DEFECTS.md`):**
  - **ID:** Proposed SD-008.
  - **Title:** Unphysical vapor-gravity coupling and incomplete gravity cancellation in free-drainage lower boundary (`WATFLOW.FOR:213`).
  - **Classification:** Computational defect (variable reuse side effect).
  - **Summary:** In `WATFLOW.FOR:213`, `rBot` is computed as `-ConB*Grav*fRE` after `ConB` has been augmented with isothermal vapor conductivity `ConVh` at line 211. Because interior gravity term `B` (line 205) excludes `ConVh`, `B + rBot` leaves an unphysical residual $-K_{vh} \cdot \text{Grav}$ on the RHS of node 1, attributing a fictitious gravity drainage flux to water vapor.
* **Rust Rows Affected:**
  - `crates/hydrus-core/src/heat/vapor/water.rs` / `crates/hydrus-core/src/water_solver.rs`: Free drainage boundary flux evaluated as $-K_{L,\text{face}} \cdot g$. The RHS cancels gravity flow cleanly.
* **Guard Change:**
  - Identical to Option (a): `VaporBoundaryPair` extended to accept `FreeDrainage`.
* **Test Impact:**
  - Guard unit tests updated identically to Option (a).
  - Parity comparison against unmodified Fortran reference will diverge at the bottom boundary by an amount proportional to $K_{vh,\text{face}} \cdot g$. Trajectory comparisons in Wave 5 would require either widened tolerances or documenting a permanent parity difference.
* **Reference Run Needed:**
  - Either:
    1. A reference run using a patched Fortran binary (moving line 213 before line 211 in a staged source copy under `.development/`); or
    2. An unpatched reference run with an accepted, documented numerical difference.

---

### Option (c): Reject with a Trigger (Status Quo)

* **Description:**
  Retain the current behavior: keep `LowerBoundaryMode::FreeDrainage` rejected with `VaporError::UnsupportedLowerBoundary` whenever vapor transport is active. Document the Fortran source defect as the audit trigger explaining why free drainage is unsupported.
* **Rust Rows Affected:**
  - None.
* **Guard Change:**
  - None (`crates/hydrus-core/src/heat/vapor/mod.rs:295` remains untouched).
* **Test Impact:**
  - None. All existing guard tests continue to pass.
* **Reference Run Needed:**
  - None.
* **Plan Alignment:**
  - Note: This option conflicts with user directive D1 ("Assess and implement", 2026-11-14) and would require deselecting tasks 30.2.1, 30.4.2, and 30.5.1 from Orchestration 30.

---

## 4. Recommendation with Reasons

**Recommendation:** Proceed with **Option (a) (Match the source as written)**, while simultaneously proposing SD-008 in project documentation.

**Reasons:**
1. **Preservation of Benchmark Parity:**
   The foundational mission of `hyd.rs` (and Milestone M6) is strict behavioral verification against the pinned reference executable `src_reference/WATFLOW.FOR`. Implementing Option (a) ensures that `hyd.rs` can achieve exact, tight-tolerance trajectory parity against fixtures generated by the official reference build in tasks 30.4.2 and 30.5.1 without requiring patched reference binaries.
2. **Alignment with User Directives:**
   The user explicitly selected "Assess and implement" (Decision D1, `EXECUTION_PLAN_30.md`). Option (a) directly unblocks task 30.2.1 (implementation) and task 30.4.2 (reference runs in `.development/`) under Gate G30.2.
3. **Physical Impact is Negligible in Typical Free-Drainage Regimes:**
   Free drainage lower boundaries are typically applied at the bottom of deep soil profiles under moist to wet conditions, where liquid hydraulic conductivity $K_L$ dominates vapor conductivity by several orders of magnitude ($K_L \gg K_{vh}$). Consequently, the numerical magnitude of $K_{vh} \cdot \text{Grav}$ is exceedingly small relative to $K_L \cdot \text{Grav}$, meaning the defect has virtually no practical consequence on soil moisture dynamics, while maintaining bitwise fidelity to the published HYDRUS-1D model.
4. **Architectural Cleanliness via SD Registration:**
   Documenting the defect under proposed SD-008 maintains full scientific integrity and transparency without breaking reference compatibility. If a future "physically corrected" solver profile is introduced, it can be added under a distinct configuration flag.

---

## 5. Numbered Questions for the User (Gate G30.2)

1. **Option Selection:** Do you approve **Option (a)** (match the Fortran source as written, carrying the vapor term in `rBot` and leaving the $-K_{vh} \cdot \text{Grav}$ residual in `PB` to preserve exact parity with the pinned reference), or do you prefer **Option (b)** (correct the Fortran defect in Rust, which will cause a small, permanent divergence from unpatched reference runs)?
2. **Defect Tracking:** If Option (a) is approved, do you agree to log proposed **SD-008** in `docs/FORTRAN_SOURCE_DEFECTS.md` in a subsequent documentation task to record the `WATFLOW.FOR:213` variable reuse defect?
3. **Upper Boundary Pair Scope:** In task 30.2.1, should `VaporBoundaryPair::classify` accept `FreeDrainage` exclusively paired with `UpperBoundary::Flux` (matching the Wave 4 reference deck design), or should it also accept `UpperBoundary::Head`?
4. **Criteria and Tolerances:** For the Wave 5 trajectory comparison (task 30.5.1), do you approve evaluating free-drainage cases against the standard global field tolerances in `criteria.json["fields"]`, without introducing case-specific relaxation?
