#!/usr/bin/env bash
# Fail if `unsafe` code appears anywhere except firmware/dt-hal, the one crate
# that wraps the C HAL (REQ-LANG-03). Each other crate also has
# #![forbid(unsafe_code)]; this check additionally catches the attribute being
# removed.
set -euo pipefail
cd "$(dirname "$0")/.."

pattern='\bunsafe[[:space:]]*(\{|fn\b|impl\b|trait\b|extern\b)'
hits=$(grep -rnE "$pattern" --include='*.rs' crates firmware \
         --exclude-dir=target --exclude-dir=.embuild \
       | grep -v '^firmware/dt-hal/' || true)

missing=""
for lib in crates/*/src/lib.rs firmware/dt-firmware/src/main.rs; do
    grep -q '^#!\[forbid(unsafe_code)\]' "$lib" || missing+="  $lib"$'\n'
done

status=0
if [[ -n "$hits" ]]; then
    echo "error: unsafe code outside firmware/dt-hal:" >&2
    echo "$hits" >&2
    status=1
fi
if [[ -n "$missing" ]]; then
    echo "error: missing #![forbid(unsafe_code)] in:" >&2
    printf '%s' "$missing" >&2
    status=1
fi
[[ $status -eq 0 ]] && echo "ok: unsafe code is confined to firmware/dt-hal"
exit $status
