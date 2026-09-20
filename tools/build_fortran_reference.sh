#!/bin/sh
set -eu

usage() {
  echo "usage: $0 --build-dir <directory below .development>" >&2
  exit 2
}

[ "$#" -eq 2 ] && [ "$1" = "--build-dir" ] || usage
build_dir=$2
case "$build_dir" in
  .development/*) ;;
  *) echo "error: --build-dir must be below .development/" >&2; exit 2 ;;
esac

metadata="$build_dir/staging-manifest.json"
source_dir="$build_dir/src_reference"
executable="$build_dir/hydrus-reference"
compatibility_source="$build_dir/gnu_compatibility.FOR"
[ -f "$metadata" ] || { echo "error: missing staging metadata; run stage_fortran_reference.py first" >&2; exit 2; }
[ -d "$source_dir" ] || { echo "error: missing staged source; run stage_fortran_reference.py first" >&2; exit 2; }
command -v gfortran >/dev/null 2>&1 || { echo "error: GNU Fortran (gfortran) is required" >&2; exit 2; }

flags='-std=legacy -ffixed-line-length-none'
# These legacy Intel helper entry points remain in the pinned source. Generate a
# build-local GNU implementation rather than modifying the immutable snapshot.
cat > "$compatibility_source" <<'EOF'
      integer function NARGS()
      NARGS=command_argument_count()+1
      return
      end

      subroutine GETDAT(iYear,iMonth,iDay)
      integer*2 iYear,iMonth,iDay
      integer values(8)
      call date_and_time(values=values)
      iYear=values(1)
      iMonth=values(2)
      iDay=values(3)
      return
      end

      subroutine GETTIM(iHours,iMins,iSecs,i100th)
      integer*2 iHours,iMins,iSecs,i100th
      integer values(8)
      call date_and_time(values=values)
      iHours=values(5)
      iMins=values(6)
      iSecs=values(7)
      i100th=values(8)/10
      return
      end
EOF
# The source snapshot is fixed-form and must use precisely these portability flags.
gfortran -std=legacy -ffixed-line-length-none "$source_dir"/*.FOR "$compatibility_source" -o "$executable"

python3 - "$metadata" "$executable" "$flags" "$@" <<'PY'
import datetime as dt
import hashlib
import json
import os
import platform
import subprocess
import sys
from pathlib import Path

metadata_path, executable, flags, *command_args = sys.argv[1:]
staging = json.loads(Path(metadata_path).read_text(encoding="utf-8"))
compiler = subprocess.run(["gfortran", "--version"], check=True, text=True, capture_output=True).stdout.splitlines()[0]
digest = hashlib.sha256(Path(executable).read_bytes()).hexdigest()
manifest = {
    "archive_member_byte_match": staging["archive_member_byte_match"],
    "artifact_notice_review_status": staging["artifact_notice_review_status"],
    "command": ["gfortran", "-std=legacy", "-ffixed-line-length-none", "src_reference/*.FOR", "gnu_compatibility.FOR", "-o", "hydrus-reference"],
    "executable": "hydrus-reference",
    "executable_sha256": digest,
    "flags": flags.split(),
    "gnu_fortran_version": compiler,
    "host_architecture": platform.machine(),
    "host_os": platform.platform(),
    "patch_sha256": staging["patch_sha256"],
    "source_and_archive_sha256": staging["source_and_archive_sha256"],
    "utc_date": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
}
Path(metadata_path).with_name("build-manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

echo "built $executable"
