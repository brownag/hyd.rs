# Noncanonical smoke run

This record is execution provenance only. The inputs and outputs are not a scientific fixture, baseline, parity result, or numerical validation.

## Input status

All four committed smoke inputs were copied byte-for-byte into a fresh isolated run directory. They are independently constructed and noncanonical.

## Command

`python3 tools/run_fortran_reference.py --smoke --timeout-seconds 60`

Reference invocation: `/home/andrew/AI/hyd.rs/.development/reference-build/hydrus-reference smoke-input` in `.development/reference-build/smoke-run`.

## Hashes

### Inputs

- `ATMOSPH.IN`: `eacc59021a560c7806d290d5fb5c4a2f3e6a76cbe1aa6a478d0ee76ec1b842ec`
- `LEVEL_01.DIR`: `eb4bd64f7014f7d42e9d358035802242741b974e8dfcd37c59f9c21ce29d781e`
- `PROFILE.DAT`: `0175b54f8e03e9a3b58979a2d8ea1dd8c240ee5e1a796a54b11c191fe06e556a`
- `SELECTOR.IN`: `a2a33cdf76cce93da0bf148e92875a45df3e284b23fd264a3521b60afd2fd1a2`

### Outputs

- `BALANCE.OUT`: `937e173643c8369e9769e9bf8f01fdbd62d305b4331a85299e94dd1bbe772801`
- `I_CHECK.OUT`: `30166088764b270b6a2a1022fe36bf0c4e3ea521b94319007d8c2e123118c0f8`
- `NOD_INF.OUT`: `ce45959eaf1ed1e92e3a6da2acbbd8e493604dcda4e0992f6bcecc3d84800c6b`
- `OBS_NODE.OUT`: `5bdbd99830cffb7fd4789cef7f349bedb329fb289426546380b8b33a80c38567`
- `PROFILE.OUT`: `1d7f5f50ba38f42a7f71e8df9784cc04686d0e5427096a262765e8fc2e5b3dfc`
- `RUN_INF.OUT`: `a1d034aa11ef8e918dda35805e5c8b86b2f858e78170888a3487815f2f5ecb5e`
- `T_LEVEL.OUT`: `98d38bf564b092f83c7c10b16c9497d5e9632c2a69ff5e2894a9fcef9c5a4f52`

### Captures

- `stderr.txt`: `400a35c0a4d0cfe789b3c30941e4ad4e1c4878a6962021d509ff5601350223eb`
- `stdout.txt`: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`

- patch: `622d1ea68285257e98a169f94dec83f3df1c671a9366f651e845806cb8c00d7e`
- executable: `c012c4d2c76107fd189229547c6583112d95e3535e0abcfe52f8f6edc33f3a1a`

## Environment

- GNU Fortran: `GNU Fortran (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0`
- OS: `Linux-7.0.0-31-generic-x86_64-with-glibc2.39`
- architecture: `x86_64`
- flags: `-std=legacy -ffixed-line-length-none`
- Python: `3.12.3`

## Exit status

`0` within the requested 60-second timeout.

## Completion evidence

`I_CHECK.OUT`, `RUN_INF.OUT`, `T_LEVEL.OUT`, `NOD_INF.OUT`, `OBS_NODE.OUT`, and `PROFILE.OUT` each contain the legacy `end` marker; `BALANCE.OUT` contains its final `Calculation time` record; the executable returned zero.

## Output inventory

- `BALANCE.OUT`: `937e173643c8369e9769e9bf8f01fdbd62d305b4331a85299e94dd1bbe772801`
- `I_CHECK.OUT`: `30166088764b270b6a2a1022fe36bf0c4e3ea521b94319007d8c2e123118c0f8`
- `NOD_INF.OUT`: `ce45959eaf1ed1e92e3a6da2acbbd8e493604dcda4e0992f6bcecc3d84800c6b`
- `OBS_NODE.OUT`: `5bdbd99830cffb7fd4789cef7f349bedb329fb289426546380b8b33a80c38567`
- `PROFILE.OUT`: `1d7f5f50ba38f42a7f71e8df9784cc04686d0e5427096a262765e8fc2e5b3dfc`
- `RUN_INF.OUT`: `a1d034aa11ef8e918dda35805e5c8b86b2f858e78170888a3487815f2f5ecb5e`
- `T_LEVEL.OUT`: `98d38bf564b092f83c7c10b16c9497d5e9632c2a69ff5e2894a9fcef9c5a4f52`

## Warnings

- `Note: The following floating-point exceptions are signalling: IEEE_OVERFLOW_FLAG`

Captured stdout and stderr are retained untracked under `.development/reference-build/smoke-run/`.

## Limitations

This proves only that the patched reference executable completed this deliberately minimal, noncanonical run on this GNU Fortran/Linux environment. It establishes no physical correctness, fixture provenance, parity, tolerance, baseline, release identity, or redistribution permission.
