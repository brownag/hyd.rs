# Solute printed-output parity evidence

Read-only comparison of the six authenticated canonical Fortran/Rust pairs. This reports agreement with each retained Fortran token’s print interval; it does not establish continuum accuracy or physical validity.

## Inputs and reproducibility

- `CASES.json` SHA-256: `d3b06531c06545ca89956b95c487bb1616ace1470ba1a8c02a3f1209ff4e8acf`.
- Authenticated inventory SHA-256: `616662ba6dbfe57e07c4e59a5d231d7feda9cb38dfb0a078414182afb678219c` (also embedded in comparison JSON).
- Comparison JSON: `.development/14.3.1/printed_parity.json`; SHA-256: `d3a2f97e45536e14b3323305c032ca67e51acd5479882fcb232a20960add29ad`.
- Successful commands: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.test_solute_printed_parity` — 10 tests passed; `PYTHONDONTWRITEBYTECODE=1 python3 tools/solute_printed_parity.py --cases reference/solute_verification/CASES.json --inventory planning/master/SOLUTE_PRINTED_PARITY_INVENTORY.json --output-root .development/14.3.1` — completed for six canonical pairs; `git diff --check` — passed.
- Comparison JSON records each pair’s Fortran and Rust manifest SHA-256 and every field record: token/value, Rust value, signed Rust-minus-Fortran difference, source format, interval, location, status, and low-information flag.
- Only canonical pairs were compared. The 36 supplemental records and all failed validation rows remain excluded. The inventory authenticates input artifact hashes and byte counts; the comparator revalidated retained artifacts before reading them.

## Per-example, field, and species coverage

Counts aggregate every comparable node/profile record in the canonical schedule; the detailed record for each value is in the JSON. `—` denotes a field without species indexing.

| Example | Field | Species | Pass | Fail | Unavailable | Low information | Unavailable reason |
|---|---|---:|---:|---:|---:|---:|---|
| `reaction_distinct` | `Sorb` | S1 | 0 | 0 | 297 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `reaction_distinct` | `Sorb` | S2 | 0 | 0 | 297 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `reaction_distinct` | `Sorb` | S3 | 0 | 0 | 297 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `reaction_distinct` | `concentration` | S1 | 9 | 288 | 0 | 0 | — |
| `reaction_distinct` | `concentration` | S2 | 9 | 288 | 0 | 9 | — |
| `reaction_distinct` | `concentration` | S3 | 18 | 279 | 0 | 9 | — |
| `reaction_distinct` | `head` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal head field |
| `reaction_distinct` | `hydraulic_conductivity` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal K field |
| `reaction_distinct` | `kappa` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal Kappa field |
| `reaction_distinct` | `moisture_capacity` | — | 0 | 0 | 297 | 0 | Rust observations do not retain the Fortran C field |
| `reaction_distinct` | `sink` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal sink field |
| `reaction_distinct` | `temperature` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped temperature field |
| `reaction_distinct` | `theta` | — | 297 | 0 | 0 | 0 | — |
| `reaction_distinct` | `v_over_ks_top` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped v/KsTop field |
| `reaction_distinct` | `water_flux` | — | 0 | 0 | 297 | 0 | Rust face fluxes do not establish the Fortran nodal Flux mapping |
| `reaction_equal` | `Sorb` | S1 | 0 | 0 | 297 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `reaction_equal` | `Sorb` | S2 | 0 | 0 | 297 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `reaction_equal` | `Sorb` | S3 | 0 | 0 | 297 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `reaction_equal` | `concentration` | S1 | 9 | 288 | 0 | 0 | — |
| `reaction_equal` | `concentration` | S2 | 9 | 288 | 0 | 9 | — |
| `reaction_equal` | `concentration` | S3 | 9 | 288 | 0 | 9 | — |
| `reaction_equal` | `head` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal head field |
| `reaction_equal` | `hydraulic_conductivity` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal K field |
| `reaction_equal` | `kappa` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal Kappa field |
| `reaction_equal` | `moisture_capacity` | — | 0 | 0 | 297 | 0 | Rust observations do not retain the Fortran C field |
| `reaction_equal` | `sink` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal sink field |
| `reaction_equal` | `temperature` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped temperature field |
| `reaction_equal` | `theta` | — | 297 | 0 | 0 | 0 | — |
| `reaction_equal` | `v_over_ks_top` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped v/KsTop field |
| `reaction_equal` | `water_flux` | — | 0 | 0 | 297 | 0 | Rust face fluxes do not establish the Fortran nodal Flux mapping |
| `reaction_zero` | `Sorb` | S1 | 0 | 0 | 297 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `reaction_zero` | `Sorb` | S2 | 0 | 0 | 297 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `reaction_zero` | `Sorb` | S3 | 0 | 0 | 297 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `reaction_zero` | `concentration` | S1 | 297 | 0 | 0 | 0 | — |
| `reaction_zero` | `concentration` | S2 | 297 | 0 | 0 | 0 | — |
| `reaction_zero` | `concentration` | S3 | 297 | 0 | 0 | 297 | — |
| `reaction_zero` | `head` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal head field |
| `reaction_zero` | `hydraulic_conductivity` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal K field |
| `reaction_zero` | `kappa` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal Kappa field |
| `reaction_zero` | `moisture_capacity` | — | 0 | 0 | 297 | 0 | Rust observations do not retain the Fortran C field |
| `reaction_zero` | `sink` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped nodal sink field |
| `reaction_zero` | `temperature` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped temperature field |
| `reaction_zero` | `theta` | — | 297 | 0 | 0 | 0 | — |
| `reaction_zero` | `v_over_ks_top` | — | 0 | 0 | 297 | 0 | Rust observations do not retain a mapped v/KsTop field |
| `reaction_zero` | `water_flux` | — | 0 | 0 | 297 | 0 | Rust face fluxes do not establish the Fortran nodal Flux mapping |
| `diffusion_cosine` | `Sorb` | S1 | 0 | 0 | 1105 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `diffusion_cosine` | `concentration` | S1 | 665 | 440 | 0 | 0 | — |
| `diffusion_cosine` | `head` | — | 0 | 0 | 1105 | 0 | Rust observations do not retain a mapped nodal head field |
| `diffusion_cosine` | `hydraulic_conductivity` | — | 0 | 0 | 1105 | 0 | Rust observations do not retain a mapped nodal K field |
| `diffusion_cosine` | `kappa` | — | 0 | 0 | 1105 | 0 | Rust observations do not retain a mapped nodal Kappa field |
| `diffusion_cosine` | `moisture_capacity` | — | 0 | 0 | 1105 | 0 | Rust observations do not retain the Fortran C field |
| `diffusion_cosine` | `sink` | — | 0 | 0 | 1105 | 0 | Rust observations do not retain a mapped nodal sink field |
| `diffusion_cosine` | `temperature` | — | 0 | 0 | 1105 | 0 | Rust observations do not retain a mapped temperature field |
| `diffusion_cosine` | `theta` | — | 1105 | 0 | 0 | 0 | — |
| `diffusion_cosine` | `v_over_ks_top` | — | 0 | 0 | 1105 | 0 | Rust observations do not retain a mapped v/KsTop field |
| `diffusion_cosine` | `water_flux` | — | 0 | 0 | 1105 | 0 | Rust face fluxes do not establish the Fortran nodal Flux mapping |
| `scalar_robin` | `Sorb` | S1 | 0 | 0 | 2193 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `scalar_robin` | `concentration` | S1 | 807 | 1386 | 0 | 0 | — |
| `scalar_robin` | `head` | — | 0 | 0 | 2193 | 0 | Rust observations do not retain a mapped nodal head field |
| `scalar_robin` | `hydraulic_conductivity` | — | 0 | 0 | 2193 | 0 | Rust observations do not retain a mapped nodal K field |
| `scalar_robin` | `kappa` | — | 0 | 0 | 2193 | 0 | Rust observations do not retain a mapped nodal Kappa field |
| `scalar_robin` | `moisture_capacity` | — | 0 | 0 | 2193 | 0 | Rust observations do not retain the Fortran C field |
| `scalar_robin` | `sink` | — | 0 | 0 | 2193 | 0 | Rust observations do not retain a mapped nodal sink field |
| `scalar_robin` | `temperature` | — | 0 | 0 | 2193 | 0 | Rust observations do not retain a mapped temperature field |
| `scalar_robin` | `theta` | — | 2193 | 0 | 0 | 0 | — |
| `scalar_robin` | `v_over_ks_top` | — | 0 | 0 | 2193 | 0 | Rust observations do not retain a mapped v/KsTop field |
| `scalar_robin` | `water_flux` | — | 0 | 0 | 2193 | 0 | Rust face fluxes do not establish the Fortran nodal Flux mapping |
| `example3` | `Sorb` | NH4 | 0 | 0 | 4004 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `example3` | `Sorb` | NO2 | 0 | 0 | 4004 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `example3` | `Sorb` | NO3 | 0 | 0 | 4004 | 0 | Fortran nonequilibrium Sorb is not equivalent to Rust equilibrium storage |
| `example3` | `concentration` | NH4 | 2621 | 1383 | 0 | 2620 | — |
| `example3` | `concentration` | NO2 | 1933 | 2071 | 0 | 1933 | — |
| `example3` | `concentration` | NO3 | 1911 | 2093 | 0 | 1896 | — |
| `example3` | `head` | — | 0 | 0 | 4004 | 0 | Rust observations do not retain a mapped nodal head field |
| `example3` | `hydraulic_conductivity` | — | 0 | 0 | 4004 | 0 | Rust observations do not retain a mapped nodal K field |
| `example3` | `kappa` | — | 0 | 0 | 4004 | 0 | Rust observations do not retain a mapped nodal Kappa field |
| `example3` | `moisture_capacity` | — | 0 | 0 | 4004 | 0 | Rust observations do not retain the Fortran C field |
| `example3` | `sink` | — | 0 | 0 | 4004 | 0 | Rust observations do not retain a mapped nodal sink field |
| `example3` | `temperature` | — | 0 | 0 | 4004 | 0 | Rust observations do not retain a mapped temperature field |
| `example3` | `theta` | — | 4004 | 0 | 0 | 0 | — |
| `example3` | `v_over_ks_top` | — | 0 | 0 | 4004 | 0 | Rust observations do not retain a mapped v/KsTop field |
| `example3` | `water_flux` | — | 0 | 0 | 4004 | 0 | Rust face fluxes do not establish the Fortran nodal Flux mapping |

## Canonical pair identity and measured bounds

| Example | Fortran manifest SHA-256 | Rust manifest SHA-256 | Common schedule |
|---|---|---|---|
| `reaction_distinct` | `0ba6e815f0f7128113211ee896f78d1b27275ba911ae3fcbdafa824eb6c94485` | `b703d7b35de47eb378fac29560b03bd9fdbae8d3ddf05229cffa99a54e9077d4` | 9 nodes; 3 species; 33 common profiles; 129 common endpoints; 128 steps |
| `reaction_equal` | `bfb91022abe91cb4783dc0a6ee38e9a615bc60ecd5788790ca75995d0a2d3fe8` | `7ec4d43de8ede0aa709d5b48b4f28d58bedc0c91e87e9f751f00d75fd759b4f1` | 9 nodes; 3 species; 33 common profiles; 129 common endpoints; 128 steps |
| `reaction_zero` | `90ac8302bed5b64afeaac4b382fe94ba29bf7c2a9a5292f99a50ec98694d27f7` | `96bbce7b694b933a8803137fc23ad7b69d8e6956ddaaf9a5f60ab6b1954f654e` | 9 nodes; 3 species; 33 common profiles; 129 common endpoints; 128 steps |
| `diffusion_cosine` | `63160c89f6f27f39923c99181a229671d87023342e3dece3b20fcb6fd5b77893` | `bed3099a49394176e3aba0dfb799c6ca7c713b7b64755407636a25845243e914` | 65 nodes; 1 species; 17 common profiles; 4097 common endpoints; 4096 steps |
| `scalar_robin` | `ac7476db12c6e18ff8c3b5a091c2867145cd3212bd8d7d09f8190ea7a406688a` | `af973d1ad19f97f79ba4d45e0a6a28a876081e23cee91765095b387dd0f943c1` | 129 nodes; 1 species; 17 common profiles; 257 common endpoints; 256 steps |
| `example3` | `8deb39852934456517baf11509876ba59b7065ec0079c8256ad010c5ee7154e5` | `43f27dd2acd665dd4596e0c0cb5fd88952aa4f413f931ef6641765d1ac91fbd2` | 1001 nodes; 3 species; 4 common profiles; 0 common endpoints; 2000 steps |

The directly comparable fields are `theta` at every common profile node and `concentration` at every common profile node/species. Their individual status is `pass` or `fail`; JSON retains exact values and each token-derived print interval. `head`, `moisture_capacity`, `hydraulic_conductivity`, `water_flux`, `sink`, `kappa`, `v_over_ks_top`, and `temperature` are unavailable because Rust observations do not retain a proven corresponding field/mapping. `Sorb` is unavailable because Fortran nonequilibrium `Sorb` is not equivalent to Rust equilibrium storage. These unavailable records are semantic/output coverage limits, not failed retained validation; canonical inputs passed authentication.

Example 3 uses only its canonical bridge pair. Direct Fortran/Rust printed-output coverage is theta at 4 common profiles across 1001 nodes, and concentration for NH4, NO2, and NO3 at those profile nodes (12,012 concentration records). This pair has zero common endpoints and 2000 steps. Its per-species pass/fail and low-information counts are in the table, and all raw values and intervals are in JSON. Earlier same-solver bridge claims are separate evidence and are not counted as direct cross-language parity. Failed Example 3 endpoint-order rows remain excluded.

## Interpretation and conditional follow-up

Printed intervals follow each source token and field format; `low_information` is reported independently and never changes pass/fail. The JSON retains signed differences even when a field fails or the interval is too wide to be informative. A parity pass means only that the retained Rust binary64 value falls inside the interval for the retained Fortran print.

No retained-data gap warrants conditional Wave 4. All six canonical pairs have authenticated, matched schedules. Comparable fields are measured directly; unavailable fields are unsupported Rust observations or incompatible `Sorb` semantics, which a new retained pair alone cannot resolve. Supplemental and failed artifacts do not provide a validated missing counterpart. The observed concentration failures are parity discrepancies, not missing coverage. Record the conditional task as `skipped_no_remediable_gap`; any future work requiring solver output or changed field mappings needs a separately scoped, explicitly approved plan.
