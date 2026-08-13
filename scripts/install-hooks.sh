#!/usr/bin/env bash
# Install this repo's git hooks. Run once per clone.
#
# ⛔ WHY THIS EXISTS. `scripts/check-privacy.sh` was written first and was only
# ever run BY HAND. That works exactly as long as whoever is working remembers
# it, and an agent's discipline resets every session while a hook's does not.
# The one repo where forgetting matters most is this one: its documents are
# extracted from a personal journal, and the leak vector is not a secret but a
# real example copied in because a real example was in front of the writer.
#
# ⛔ AND AN INSTALLER IS PROVEN BY A REAL PUSH, NOT BY A SYNTAX CHECK. A hook
# that is present, executable and syntactically valid is indistinguishable on
# disk from one that works; a broken one can even print the guard's own output
# while exiting non-zero, which git reads as "refuse to push" and which looks
# exactly like the guard doing its job. After running this, prove BOTH
# directions: a clean push must succeed, and a deliberately leaky file must be
# refused. One of those alone proves nothing — a hook that refuses everything
# passes the block test, and a hook that does nothing passes the pass test.
set -euo pipefail
cd "$(dirname "$0")/.."

hooks=$(git rev-parse --git-path hooks)
mkdir -p "$hooks"
target="$hooks/pre-push"

cat > "$target" <<'HOOK'
#!/usr/bin/env bash
# Refuse to push private material out of this repo. Installed by
# scripts/install-hooks.sh; the check itself is scripts/check-privacy.sh, which
# is the single owner of what counts as a leak here.
set -uo pipefail
repo=$(git rev-parse --show-toplevel)
if [ -x "$repo/scripts/check-privacy.sh" ]; then
  if ! "$repo/scripts/check-privacy.sh"; then
    echo "pre-push: refusing — fix the findings above, or rewrite the example." >&2
    exit 1
  fi
fi
exit 0
HOOK

chmod +x "$target"
echo "installed $target"
echo "⚠ now prove it BOTH ways: a clean push must pass, a planted leak must be refused."
