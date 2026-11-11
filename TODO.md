# TODO

## Goal

Plan and, after this plan is separately approved, rerun only the affected Robin spatial-refinement campaign with the fixed upstream-weighting mapping, preserving all pre-fix evidence and applying the campaign's valid approval, producer-pin, bounds, and evidence gates.

## Tasks

### 1. Reconcile authorization and evidence contract (read-only)
- [x] Record the authorized scope as Robin spatial refinement only; exclude Robin time, other cases, holdouts, thresholds, and unrelated campaigns.
- [x] Inventory the historical Robin-space batch and its recipe, launch ledger, inputs, outputs, verification receipt, classifications, and hashes without changing them.
- [x] Compare current fixed-mapping producer hashes with the historical recipe pins and signed Gate B review; state clearly which old artifacts remain historical and whether current lifecycle validation can accept new producer identities.
- [x] Check the current Robin-space case rows, including declared upstream weighting, launch IDs, meshes, timestep, roles, and counts; verify that only the eligible study/control rows are candidates and no holdout is included.

### 2. Establish rerun gates before any solver start
- [x] Verify whether existing signed Robin bounds remain mathematically applicable to the fixed-mapping runs; check the exact launch coverage, metrics, uncertainty derivation, producer/cases bindings, and review authorization.
- [x] Because the existing bounds/review do not authorize the new producer identity, obtain explicit user authorization for a fresh producer-bound bounds/review package and its Gate B signature before any campaign run; do not infer a re-signature or alter signed files.
- [x] Prepare a new Robin-only bounds package in isolated scratch, without editing historical bounds/reviews; bind it to current producer hashes and casebook SHA and document the reused derivation plus conditions that invalidate it.
- [x] Prepare a fresh unsigned Gate B review draft that binds the new bounds, producers, cases, and derivation evidence; stop for user signature before campaign execution.
- [x] User reviews and signs the new Robin-only Gate B review; this is mandatory before any campaign solver start.
- [x] Specify a fresh, isolated output root and lifecycle identity for the rerun; ensure no historical recipe, ledger, batch directory, classification, or raw artifact can be overwritten or reused as new evidence.
- [x] Specify exact commands and ordering for eligible Robin-space execution, verification, and classification, with stop-on-first-failure/no-automatic-retry behavior and applicable run/resource limits taken from current campaign records.
- [x] Define acceptance checks: producer and case hashes, request selector propagation, lifecycle receipts, bounds/review validation, per-launch verification, exact finding inventory, and explicit separation of new findings from pre-fix classifications.

### 3. Execute only after separate plan approval and all gates pass
- [x] Recheck repository status and all required input hashes immediately before execution; confirm the fresh output root is empty and distinct from historical evidence.
- [x] Run only the approved Robin spatial-refinement rows in the isolated root; stop on any pin, bounds, lifecycle, timeout, or verification error and preserve partial outputs without retrying.
- [x] Verify every completed batch using the campaign's evidence verifier; do not run any time group, other case, holdout, threshold, or unrelated batch.

### 4. Report and preserve the result
- [x] Write a new dated rerun report with commands, exit statuses, launch/batch IDs, producer/case/bounds/review hashes, verification outcomes, classification counts, and limitations.
- [x] Keep all pre-fix artifacts and their original classifications unchanged; describe any new result as post-fix evidence and do not silently replace or reclassify historical findings.
- [x] Update the relevant campaign state/summary only after new evidence is verified, retaining the historical record and clearly distinguishing the rerun's scope and outcome.

## Notes

- Governing context: `planning/EXECUTION_PLAN_23.md`, `planning/STATE_23.md`, `planning/master/M5V_BOUNDS_23.md`, `planning/master/M5V_FINAL_23.md`, `planning/master/M5V_REVIEW_23_NEXT_STEPS.md`, `planning/master/ROBIN_UPSTREAM_MAPPING_INVESTIGATION.md`, `planning/master/SOLUTE_VERIFICATION_SPEC.md`, and `reference/solute_verification/CASES_V2.json`.
- The mapping fix is documented as carrying `chemistry.upstream_weighting` through new V2 public `FortranCompatible` requests. Current changed producer sources include `tools/run_solute_verification_reference.py`, `tools/solute_benchmark_lifecycle.py`, `tools/solute_verification_metrics.py`, and `crates/hydrus-core/tests/solute_verification_harness.rs`; their recorded hashes are in the mapping investigation. Confirm live hashes before any run.
- The old Robin-space batch `batch-ad3ac50b1a8243b9836b99ffcd6e2f9f` and its 40 failed / 150 inconclusive classifications describe the pre-fix mapping. Preserve them as historical evidence; never overwrite, delete, or reclassify them.
- The existing combined bounds SHA is `3ac9fd8a45018d850da20fefc2877093f8fcf73910b4a7133a54ec8a039a6ee4`; the signed combined review SHA is `93db9cfceadb55eedd33b54771bc78d60d79bed553763d09e72fab8c3f2e15b1`. They bind to the prior CASES and producer identity. Applicability to fixed-mapping outputs must be established rather than assumed.
- The user's 2026-11-11 authorization permits a rerun of the affected Robin spatial-refinement campaign subject to the existing campaign gates. This plan approval is a separate required gate; it does not waive a needed bounds/review re-authorization. No campaign solver start occurs before both plan approval and all applicable evidence gates.
- Task 1 confirmed: the historical recipe uses CASES SHA `420af7c0...15df7` and old producer pins. Current mismatches are `run_solute_verification_reference.py` (old `27cbafd8...`, current `f4b8ead0...`), `solute_benchmark_lifecycle.py` (old `325935bf...`, current `232a62ac...`), and `solute_verification_metrics.py` (old `609a18e0...`, current `adf317c2...`). The historical signed Gate B review and combined bounds bind those old producer hashes. Current lifecycle `verify-only` also enforces historical source identity, so it cannot validate old recipes under changed producers; preserve existing receipt as historical.
- Historical batch `batch-ad3ac50b1a8243b9836b99ffcd6e2f9f` uses launch IDs 129/193/257-node study rows at `dt=0.0078125` plus 257-node control at `dt=0.00390625`; each row has a 60-second timeout. Its activation, recipe, results, classification, reference receipts, summary ledger, and `robin-space.verify.log` were inspected read-only. Classification SHA is `b1dd3b199461b325209a084ef9e32b12c0ee357ececdfdf5da8e5399b12df38c`; verify-log SHA is `1f67046e03032f4b07c06da7c994af314037e5e7a92637ad936ea94e6b02f4ae`; verify status is `evidence_verified` for the historical source identity.
- Current casebook still declares `scalar_robin.chemistry.upstream_weighting=true`; the four eligible space study/control rows above are `role: study`/control as recorded, `axis: space`, and no holdout. New V2 request mapping carries the true selector. The three study results and one control must all be rerun together for the spatial refinement and its control comparison.
- Although the signed bounds contain all four matching launch IDs and their values/derivation are unchanged, the combined bounds and review producer maps bind to the three pre-fix source hashes. The review authorization is specifically the 2026-11-09 Gate B sign-off for that map. The lifecycle verifier rejects source identity changes. Therefore do not reuse these bounds/review for post-fix verification/classification without a separately authorized producer-bound review; this is the current stop gate.
- User explicitly authorized preparation of a fresh producer-bound bounds/review package and its Gate B signature process on 2026-11-11. This authorizes package preparation only, not any solver start. The retained derivation has 190 entries per row; all four rows use scale S=1, the fixed timestep schedules, and arithmetic gamma_N*S. The draft must condition use on fresh observations confirming identical fixed-step counts and all metric scales <=1; otherwise stop and derive new bounds/review.
- Proposed bounds report: `planning/master/M5V_ROBIN_RERUN_BOUNDS.md`. New scoped bounds: `.development/23-robin-rerun/bounds-robin.json`, SHA-256 `d29475495cbc147675857155feaef9d1db2b2d64a97c03d1385164c001490ae3`. The review draft was initially unsigned (SHA-256 `58ac8fc4e0cca09630913b01d7592dc435b2af5b1e3637d91b7c9961b17a4b1b`) and was subsequently signed as recorded below. All quadrature entries are deliberately null; no historical empirical sensitivity is carried forward.
- Gate B approved 2026-11-11. The signed review is `.development/23-robin-rerun/review-robin-draft.json`, SHA-256 `d476498ec4cdf58bfbf053922cac2db45f269d8b205c31a8f3514953060970f1`. It authorizes only the four-row bounds scope and stated validity conditions; it does not waive remaining run preflight gates.
- Proposed fresh lifecycle root is `.development/23-classify-3/`, a registered convergence task identity that does not currently exist. It is separate from the historical `.development/23-final/` and `.development/23-controls-8/` roots. Create/use it only after Gate B signature and remaining execution approval gates.
- Proposed execution order after Gate B: prepare a fresh activation for exactly the four launch IDs, run one `--execute` batch with `--bounds-source .development/23-robin-rerun/bounds-robin.json --bounds-review .development/23-robin-rerun/review-robin-draft.json` in output root `.development/23-classify-3/`, using only authenticated cached references from `.development/23-final/reference-cache` (no `--allow-reference-work`; stop if cache coverage is insufficient), then verify that exact new batch with `--verify-only --batch <new-batch-path>`. Each row has a 60-second ceiling. Stop on first lifecycle, pin, bounds, timeout, schedule, scale, or verification error; no resume or retry. The final review file must be user-signed (nonempty authorization) and its final hash used in the activation/recipe.
- Execution completed in `.development/23-classify-3/batch-25a12c50ed6a48b7a80d896512de505e`; verification returned `evidence_verified`. The complete commands, run IDs, hashes, and result counts are in `planning/master/M5V_ROBIN_RERUN_23.md`.

## Discovered Tasks

- [x] Resolve the analytical reference-cache identity mismatch before solver execution. The existing scalar-Robin cache identifies `tools/solute_verification_metrics.py` with pre-fix SHA `609a18e0...`; current SHA is `adf317c2...`, and no current-identity reference cache was found. User authorized regenerating only the three scalar-Robin analytical references needed for meshes 129, 193, and 257 (the control reuses the 257-node reference) under `.development/23-classify-3/reference-cache`, with no artificial timeout and no Rust/Fortran solver starts. Do not bypass cache authentication or start campaign solvers until generation and cache admission are verified.
- Reference regeneration completed for 129/193/257 meshes; all three admission statuses are `admitted`, schema `solute-metrics-reference-v1`; no Rust/Fortran solver was launched. Read-only preflight then confirmed the signed review, current 11-producer map, casebook SHA, exact four-row scope, admitted current-identity references, and a fresh batch/ledger root.
- The existing `TODO.md` was replaced at the user's explicit request; its previous contents remain available in Git history. Preserve unrelated working-tree changes, including the user's edits to `README.md` and Robin planning documents.

## Follow-up: read-only Robin diagnostic analysis

### Goal

Assess whether the post-fix Robin-space error reversals are large and coherent enough to indicate meaningful refinement behavior, using only already-verified artifacts and without changing campaign evidence or launching runs.

### Tasks

- [x] Read the existing verified post-fix classification, result metrics, raw observations, references, and signed uncertainty/bounds inputs for the four authorized rows; do not modify them.
- [x] For the 39 failed findings, calculate and compare signed errors and absolute errors at each mesh, adjacent-mesh changes, reference/solution scales, and available uncertainty intervals; distinguish zero/near-zero cases and report when quadrature uncertainty prevents a resolution judgment.
- [x] Group the reversals by boundary flux, interior/endpoint concentration, and inventory; identify where reversals occur across time and whether they share a pattern.
- [x] Summarize what the existing evidence supports and does not support, explicitly separating observed nonmonotonicity from causal claims about boundary conditions, inputs, discretization, or precision; propose the smallest next diagnostic that would discriminate among plausible causes.

### Constraints

- Read-only: no campaign classification, thresholds, bounds/reviews, summaries, or solver artifacts may be edited or regenerated. No solver, holdout, or unrelated campaign runs.
- Preserve existing user edits and historical pre-fix evidence. Report conclusions in chat; do not create or alter a report unless separately approved.

## Follow-up: 257-node time-step sensitivity diagnostic

### Goal

Use the already completed 257-node Robin study/control pair to estimate how much the coarser time step contributes to the observed reference error and spatial-refinement reversal, without launching additional solvers or changing campaign evidence.

### Tasks

- [x] Read-only verify the exact two 257-node run identities, configured time steps, analysis times, and existing authenticated observations/results: study `convergence-scalar_robin-space_257-dt2e-7-rfc-baseline` uses `dt=0.0078125` (`1/128`); control `convergence-scalar_robin-space_control-257-dt2e-8-rfc-baseline` uses `dt=0.00390625` (`1/256`), half the study time step. Do not infer dt from the launch-ID suffix.
- [x] At common analysis times 0.25, 0.5, and 1.0, compare the two runs' signed solution differences and reference errors for concentration profiles/norms, Robin boundary flux, and inventories; report absolute and scale-relative changes, and flag quadrature/reference uncertainty limitations.
- [x] Compare the same-mesh time-step effect with the 129→193→257 fixed-time-step spatial trend, identifying which observables/times show time-step sensitivity large enough to plausibly explain the reversal and which remain unexplained.
- [x] Summarize conclusions and the smallest next discriminating diagnostic in chat without changing classifications or recommending a tolerance based on these results alone.

### Constraints

- This plan authorizes analysis of existing artifacts only—not a new solver run. Do not edit/recompute results, observations, references, bounds/reviews, classifications, thresholds, or campaign summaries. No holdout or unrelated campaign work.
- Preserve the previous diagnostic plan and all user edits. Do not create a report file unless separately approved.

## Follow-up: third 257-node time step

### Goal

Run one **standalone exploratory** scalar-Robin diagnostic at 257 nodes with the next halved time step, `dt=0.001953125` (`1/512`), to determine whether the existing 257-node time-step sensitivity continues. This is not campaign evidence and will not be lifecycle-classified.

### Tasks

- [x] Read-only inspect the registered campaign path and the verification harness. **Outcome:** the campaign validator rejects an unregistered row and the existing Gate B review is limited to four IDs, but the existing hash-verified Rust test harness accepts a custom flat request and confines output to approved `.development/23-classify-3/` scratch. Use that route only as an exploratory run; do not make a campaign manifest, ledger entry, or classification.
- [x] Specify the one-row run: copy the existing 257-node `dt=1/256` request, preserve all physical inputs, case hashes, upstream-weighting selector, sampling interval, and analysis times, change only the unique exploratory identifiers and `dt=1/512`; this gives 512 fixed steps to t=1. Full command and checks are specified in the tasks below.
- [x] Create a unique request and output directory under `.development/23-classify-3/exploratory-robin-dt1-512/`; verify request fields/hash, executable/source identity, and that all destinations are new. Do not touch the official casebook, existing batch, or prior runs.
- [x] Run only `requested_study` in the existing hash-verified harness executable with the four required environment variables and the custom request; capture exact command, exit status, stdout/stderr, executable/request/observation hashes, and a clearly labeled exploratory provenance note. Stop on any mapping, hash, schedule, path, or execution error; do not retry.
- [x] Verify the output contains exactly 512 steps to t=1 and the expected profile times. Compare concentration fields, outlet flux, and inventory/error metrics at t=0.25/0.5/1.0 with the existing 257-node `dt=1/128` and `dt=1/256` runs, using only the already-admitted 257-node analytical reference; state any limitations if exact reference-metric derivation is unavailable.
- [x] Summarize the three-step time-sensitivity pattern in chat as exploratory evidence only. Do not classify findings, assign or change thresholds, claim Gate B evidence, or update campaign summaries.

### Constraints

- User authorized an exploratory standalone run on 2026-11-11. The run is expressly outside the registered campaign workflow; therefore do not represent it as Gate B-reviewed, lifecycle-verified, classified, or official scientific evidence.
- The existing signed Gate B review does not cover this run and must not be reused. No new bounds/review or casebook registration is planned for the exploratory comparison.
- Do not alter authoritative casebooks, producer sources, signed/historical bounds or reviews, existing results, classifications, thresholds, holdouts, or unrelated campaign evidence. Preserve all prior artifacts. No other mesh, timestep, case, or campaign run.

## Follow-up: purpose-based M6 readiness and evidence policy proposal

### Goal

Develop an explicit, purpose-based policy that can authorize M6 implementation and M6-scoped acceptance while keeping M5-V's unresolved status and the limits of current evidence visible. Use both the independent analytical reference and pinned Fortran as distinct oracle sources where applicable. Preserve historical campaign findings and identify deferred risks without implying broad fitness-for-use claims.

### Tasks

- [x] Reconcile governing M6 entry and acceptance requirements across ADR 0019, `GOALS.md`, the roadmap, M5 evidence, validation strategy, and current campaign records; state conflicts and distinguish existing policy from proposed changes.
- [x] Define the proposed authorization boundary: M5-V may remain partial/open; specify exactly what M6 implementation and M6-scoped acceptance can claim, and explicitly exclude general solute, release, or whole-algorithm acceptance unless separately evidenced.
- [x] Identify M6's in-scope user purposes, supported heat/temperature outputs, and safety-/decision-relevant observables from existing requirements. Mark unspecified purposes, outputs, or operational tolerances as unknown rather than inventing them.
- [x] Build an evidence matrix for relevant existing M5 observations: analytical-reference agreement and uncertainty, pinned-Fortran parity/paired evidence, refinement/time-step sensitivity (including the exploratory `dt=1/512` run), independent balances, and unresolved Robin/time/holdout fields. Keep exploratory evidence distinct from lifecycle-verified campaign evidence.
- [x] Propose a purpose-based acceptance/escalation rule: focus on decision-relevant outputs, justify practical ranges from use, distinguish oracle evidence, and require more runs only when they can change the implementation or supported-use claim. Do not infer limits from “small” offsets alone or retroactively alter thresholds.
- [x] Decide whether existing evidence suffices for the scoped M6 handoff; if not, specify the smallest predeclared targeted check, its provenance/oracle, measurable pass criteria, approvals, run limits, and stop conditions. Do not execute it in this planning task.
- [x] Draft the specific M5-V/M6 operational handoff as a project decision record, not an ADR; also draft a separate generic ADR for decision-recording conventions. The user accepted the M5-V result with limitations and unblocked M6 on 2026-11-11. The generic ADR remains a draft; linked M5-V/M6 status updates were then applied, preserving historical campaign outcomes.
- [x] Present the proposed policy, evidence matrix, acceptance criteria, exact edits, and follow-up risks for explicit user approval before applying governance edits or authorizing a run.

### Constraints

- This is a planning/proposal task only. No solver runs, holdouts, campaign registration/classification, or edits to casebooks, thresholds, signed bounds/reviews, historical results, or campaign summaries.
- Preserve existing user changes and prior records. **Satisfied:** the user explicitly approved accepting M5-V with limitations and unblocking M6; the dated decision and current status links now record it. The generic ADR remains unaccepted and is not required for this handoff.
- Oracle claims must remain scoped: analytical agreement is independent evidence for the mathematical case; pinned-Fortran agreement is source-snapshot parity/behavior evidence, not experimental truth or universal release compatibility.
- Do not impose a test or numerical limit merely because the campaign tooling can measure it. Each proposed gate must answer a use-relevant question; if a limit itself is unvalidated, use a bounded exploratory pilot to assess whether it is meaningful instead of treating it as an established pass/fail gate.

### Planning findings (2026-11-11)

- ADR 0019 and Orchestration 23 explicitly block M6 until M5-V holdouts pass; `GOALS.md` says the user may choose M6 planning by accepting partial M5-V status. The roadmap requires a reviewed M5-V handoff. These are inconsistent and a user-approved governance change is required before M6 is unblocked.
- Current M5-V status remains partial/open: the final campaign has 12 achieved, 40 failed and 12,888 inconclusive findings; the post-fix Robin spatial rerun has 39 nonmonotonicity failures and 151 inconclusive findings. No holdout ran and `THRESHOLDS_V2.json` does not permit one. Preserve these as findings, not blanket evidence of unfitness.
- The standalone `dt=1/512` Robin run is exploratory only. At t=1 its relative errors against the admitted analytical reference are 0.0206% concentration L-infinity, 0.0261% outlet flux and 0.00519% total inventory; all decrease across 1/128, 1/256 and 1/512. This supports timestep sensitivity for one case, not general convergence, M5 closure, or direct Fortran agreement. The analytical profile uncertainty is about 1.35e-16; exploratory inventory has no new rigorous quadrature bound.
- The 44/44 Fortran parity result is independent source-snapshot parity evidence for its fixture matrix. It is not a Fortran trajectory comparison for the exploratory Robin run, proof of physical truth, or general release compatibility.
- M6 purpose, exact supported thermal outputs, whether vapor/energy is included, and practical operational tolerances are not specified in the reviewed requirements. Do not invent them. The existing validation strategy calls for temperature, vapor/energy terms, coupling order and balance; M6 evidence must define and test its selected subset.
- **Accepted decision:** accept M5-V as-is with limitations and unblock M6 planning/implementation. Preserve all campaign classifications and the no-holdout status; no further M5 campaign is required by default. Precision/timestep interactions and uncovered areas remain caveats, to revisit only if a real supported-use case reveals a consequential discrepancy. M6 acceptance remains separate and scope-specific.
- No further run is justified merely to “clear” every M5 failure before M6. First define M6 scope and inspect whether existing M5 evidence covers the exact coupled path. If a gap remains, authorize one minimal, preregistered diagnostic on that path with frozen observables, oracle, total-error limits, independent balance criteria, producer/input hashes, resource limit and stop conditions. Do not reuse exploratory artifacts as official campaign classification or alter holdout controls.
- Governance outcome: the specific handoff is recorded in `planning/master/M5V_M6_HANDOFF_DECISION.md`; ADR 0019, its index, `GOALS.md`, roadmap, validation strategy, README, M5 evidence, review memo, and state board now link to the accepted-with-limitations disposition. No campaign artifact, holdout, threshold, or signed bounds/review was changed. Generic ADR draft: `planning/master/DECISION_RECORDING_POLICY_DRAFT.md`; not accepted or indexed.
