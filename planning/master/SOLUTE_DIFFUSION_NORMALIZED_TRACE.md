# Diffusion first-step discriminator

## Orchestration 16 task 16.3.1 result

**Status: complete; normalized first-step evidence captured.** This report
supersedes the earlier stopped 16.1.3 status below. No Rust change is proposed:
the operator difference is measured, but the evidence does not establish that
the Rust discretization is an implementation defect rather than a different
time/spatial formulation. Printed parity remains the comparison target, not a
physical or continuum accuracy claim.

### Run identity and provenance

The canonical `diffusion_cosine` case remains unchanged. Preparation 0007 made a
task-local schedule ending at `0.0625 h`: 65 nodes, S1, fixed
`dt=0.000244140625 h`, 256 steps, one profile output at `0.0625 h`. The canonical
case ends at `1 h` with outputs at `0.25`, `0.5`, and `1 h`; the derived schedule
changed only the task-local end/output times and step count. The derived setup is
recorded in
`.development/16.3.1/preparations/prepare-0007-diffusion_cosine/provenance.json`.

Authenticated identities:

- Casebook: `reference/solute_verification/CASES.json`, 73,947 bytes,
  SHA-256 `d3b06531c06545ca89956b95c487bb1616ace1470ba1a8c02a3f1209ff4e8acf`.
- Inventory: `planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json`, 546,509
  bytes, SHA-256 `616662ba6dbfe57e07c4e59a5d231d7feda9cb38dfb0a078414182afb678219c`.
- Retained Fortran request/manifest: request SHA-256
  `c5646b41687c5c51ef4aaa6a9154c4cfbe2e474210ac2c81735073fd86d959cb`,
  manifest SHA-256 `63160c89f6f27f39923c99181a229671d87023342e3dece3b20fcb6fd5b77893`.
- Retained Rust request/manifest: request SHA-256
  `d424c005d6564cd3ff7a94573ce4c41313bf5907afec6a239299cffa90399646`,
  manifest SHA-256 `bed3099a49394176e3aba0dfb799c6ca7c713b7b64755407636a25845243e914`.
- Preserved Rust trace: `.development/15.2.1/runs/rust/stderr.txt`, 21,080
  bytes, SHA-256 `6c9bf2f5593e646c1318ad902128ba6f379bbdb73f00dc6a81af1904e677fa22`.
- Canonical `SOLUTE.FOR`: 80,186 bytes, SHA-256
  `81fed6adbbd18aabe53385151166ab36d0df504a7939b54e10460c2254488910`.
- Attempt-local instrumented `SOLUTE.FOR`: 78,745 bytes, SHA-256
  `e8c0fa9069780a9d46ca28d4a25b94b645aeeb8e213e077385b0157cfaff9479`.
- Preparation wrapper: 8,663 bytes, SHA-256
  `1dddc01e63ed7c76319dac2464109468257d8eef1f27f07060d1e82f0bbf1ba6`; runner
  `.development/16.2.1/trace_runner.py`: 24,724 bytes, SHA-256
  `9173a8493c240a9fcd5f5d1bd0461376c5024743cb89b7ac66b083bd11d67f5b`.
- Task-local input hashes: `SELECTOR.IN` 1,633 bytes,
  `11fbcb630b6e27e2278f842b05679d12ec1cd1eeb9ed1d8b86db1ed1444ab5d7`;
  `PROFILE.DAT` 3,102 bytes,
  `f1ee8acfa92b72a08eacbf78836755eeb6adfc4a6259111de0172e2d59bd19ac`;
  `LEVEL_01.DIR` 2 bytes,
  `eb4bd64f7014f7d42e9d358035802242741b974e8dfcd37c59f9c21ce29d781e`.

Preparation attempts 0001–0007, build 0008, and launch 0009 are retained in
`.development/16.3.1/attempt-ledger.json`; no prior attempt was overwritten.
The reviewed preparation used
`python3 .development/16.3.1/prepare_diffusion.py --task-root .development/16.3.1`.
The build command was
`python3 .development/16.2.1/trace_runner.py build --task-root .development/16.3.1 --attempt-label diffusion-fortran-build-1`.
It used `/usr/bin/gfortran` and succeeded (binary SHA-256
`e59a6d0222a9b6474d724a508aa133d299e8fdf4d2b7711a2c50b85b8fb8d101`). The
authorized launch command was:

```sh
python3 .development/16.2.1/trace_runner.py run --task-root .development/16.3.1 --attempt-label diffusion-first-step-1
```

It exited 0 in 0.250663 s. Attempt `launch-0009-diffusion-first-step-1` records
252,362 output bytes. `stdout.log` is 22,296 bytes (SHA-256
`9b8fe3f16f1427c1307928e94100fa738c0c35cb4f574575129b97fbcb8abbb5`);
`stderr.log` is empty. The Fortran output includes the expected run outputs,
including `PROFILE.OUT` (7,314 bytes, SHA-256
`e56c60b0a1b3ab37e7772d23467452cb1f2b7f9e633a52a88e1d351784463fa4`) and
`solute1.out` (56,387 bytes, SHA-256
`f0e28ffc2ce546b23788e2f62f5459e8e3c7a33914fb8a8b111c2f7e51946905`). Every
attempt artifact hash is recorded in the ledger.

### Normalization and comparison

The instrumented call recorded `t=dt=0.000244140625 h`; the preserved Rust
first-step trace uses the same accepted timestep. There are exactly 65
`TRACE163_MATRIX` rows and 65 `TRACE163_SOLVED` values. Fortran coordinates run
from `x=-2` to `x=0`; the Rust physical coordinate increases from inlet to
outlet, so Fortran row `i` maps to Rust node `65-i` (1-based Fortran row,
0-based Rust node). With this reversal, Fortran `E,D,B` map to Rust
`lower,diagonal,upper` respectively.

For every one of the 63 interior rows, Fortran has lower and upper coefficients
`-1.37600004673` and diagonal `112.832000732`, while Rust has `-2.752`,
`115.584`, and `-2.752`. The normalized Fortran-minus-Rust coefficient
differences are consistently `+1.37599995327`, `-2.75199926758`, and
`+1.37599995327`. `MatSet` forms the Level 2 spatial matrix terms with `epsi`,
while the Level 1 RHS terms use `alf=1-epsi`; these are the two time levels in
its `NLevel=2` loop. With uniform `theta` and `D`, the observed current-level
Fortran off-diagonal magnitude is half the Rust face coefficient, consistent
with the source's time weighting. This is evidence of different
time/operator formulations, not by itself proof that Rust has an incorrect
coefficient. Interior RHS values are close but not identical: across all
interior nodes their difference ranges from `-0.0008261745` to `+0.0008277942`.

The task-local Fortran lower boundary uses `kBotCh=0`; `MatSet` applies the
free-drainage closure by setting `D(1)=-1`, `E(1)=1`, and `F(1)=0`, which is a
zero-gradient row. The physical inlet row (Rust node 0; Fortran row 65) is Fortran
`(lower=0, diagonal=56.416000366, upper=-1.376000047, RHS=68.799591064)` versus
Rust `(0,57.792,-2.752,68.8)`. At the physical outlet (Rust node 64; Fortran row
1), Fortran has `(1,-1,0;0)`, a zero-gradient closure, whereas Rust has
`(-2.752,57.792,0;41.28)`, a half-cell balance. Thus endpoint closure is a
separate operator difference, not an interior orientation artifact.

The Rust trace stores pre-solve rows only. Its first-step solution was therefore
reconstructed by solving those 65 retained tridiagonal rows in double precision;
Thomas elimination in the Rust node order was used. This is a derived value,
not a new Rust solver launch. Comparing it with the
Fortran post-`BanSol` vector after coordinate reversal gives a maximum absolute
state difference of `0.0003047828` at Rust node 64 (outlet). Other sample
differences: node 0 `+8.89025e-8`, node 1 `+8.95269e-9`, node 32
`+1.16544e-8`, and node 63 `+3.66495e-6` (Fortran minus reconstructed Rust).
The solution difference is concentrated near the outlet closure.

**Disposition: `no_fix_cause_unisolated`.** The capture establishes that the
earliest assembled operators differ in both the interior spatial contribution
and outlet closure, and that their first-step solutions differ. It does not
establish that the Rust implementation is faulty: the trace does not settle
whether the retained Fortran temporal/spatial weighting and free-drainage row
are the required translation target for Rust's formulation. Do not start
conditional correction task 16.5.1 on this evidence. Any follow-up should first
identify the intended equation/boundary mapping; no parity-based physical or
continuum claim follows.

Verification command:

```sh
python3 .development/16.2.1/trace_runner.py verify --task-root .development/16.3.1
```

Result: pass; 57 retained files, 4 aliases, 9 unique attempts, artifact hashes
verified. `git diff --check` also passes.

## Prior status (superseded)

### Authenticated case and retained evidence

The target is the authenticated 65-node `diffusion_cosine` S1 case: `theta=0.43`,
`rho=0.8 g/cm3`, `q=0`, effective `D=0.2 cm2/h`, `Kd=0.5375 cm3/g`, and zero
reaction. The first retained failing concentration profile is at `0.0625 h`.
The task-15 manifest identities and all inventory-listed request, manifest, and
output artifacts were rechecked for Orchestration 16: Fortran manifest SHA-256
`63160c89f6f27f39923c99181a229671d87023342e3dece3b20fcb6fd5b77893`; Rust
manifest SHA-256 `bed3099a49394176e3aba0dfb799c6ca7c713b7b64755407636a25845243e914`.
All 34 Fortran and 7 Rust artifacts matched inventory SHA-256 and byte counts.

The preserved Rust-only first-step trace uses `dt=0.000244140625 h`. At interior
node 32, the row is lower `-2.752`, diagonal `115.584`, upper `-2.752`, RHS
`110.08`; at endpoints the rows are `(0,57.792,-2.752;68.8)` and
`(-2.752,57.792,0;41.28)`. The mass derivative includes equilibrium sorption,
`theta+rho Kd=0.86`. These values describe Rust assembly only. The retained
profile comparison has 10/65 out-of-interval values at 0.0625 h, all with
negative Rust-minus-Fortran residuals, but this signature does not identify a
causal term.

### Missing Fortran side and task-16 run gate

Orchestration 15's bounded Fortran process stopped before matrix assembly when
Linux could not open the exact `'.\\Selector.in'` path. It produced no accepted
timestep, Fortran matrix/RHS, endpoint closure, or solved state. The Orchestration
16 inventory reauthentication succeeded. However, the existing task-15 runner
has a hard-coded `.development/15.2.1` root and enforces a two-launch cap; using
it would violate the task-16 `.development/16.1.3` and one-launch requirements.
No compliant task-16 driver and exact reviewed preflight/build/run procedure was
available in this execution. Therefore the worker stopped before creating
task-local copies, running preflight/build, or launching a solver. The
separately authorized one-run allowance remains unused; there was no retry.
Details are in `.development/16.1.3/prelaunch-ledger.md`.

**Disposition: `blocked_no_fortran_discriminator`; conditional correction:
`no_fix_cause_unisolated`.** The Rust trace and parity residuals cannot isolate a
stencil, storage weighting, endpoint, or solve defect without Fortran rows and
state. No numerical code change is supported. A future execution can proceed
only after a task-local one-launch procedure is written and reviewed against
the plan's exact path, hash, timeout, output capture, and aggregate 100 MiB cap.
Printed-output parity remains the comparison target and is not evidence of
physical or continuum accuracy.
