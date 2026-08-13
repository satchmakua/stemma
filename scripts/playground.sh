#!/usr/bin/env bash
# Build a real, explorable Asterian family and open it in the desktop explorer.
#
# WHY THIS EXISTS. The files in `fixtures/` are deliberately small: `proto_asterian`
# is an inventory with NO lexicon, and `asterian_attested` is nine hand-authored
# words, each one chosen to prove a specific engine behaviour (the feeding order, the
# three-way nasal assimilation, the word that makes rule order observable). They are
# a test harness, not a language — opening them in the explorer shows you nine words
# and is rightly underwhelming.
#
# This script does what the README's walkthrough does, in one go: coins a full
# 673-word vocabulary from the built-in concept list, forks it down three branches
# with their real sound-change histories, drifts one branch's meaning, and opens the
# result. Everything is seeded, so the same family comes back byte-for-byte every
# time — delete `out/` and re-run to prove it.
#
#   ./scripts/playground.sh            build (if needed) and open the explorer
#   ./scripts/playground.sh --rebuild  discard out/ and regenerate from scratch
#   ./scripts/playground.sh --no-ui    just build the family, print the table, exit
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

REBUILD=0
OPEN_UI=1
for arg in "$@"; do
    case "$arg" in
        --rebuild) REBUILD=1 ;;
        --no-ui)   OPEN_UI=0 ;;
        -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
        *) echo "unknown option: $arg" >&2; exit 2 ;;
    esac
done

CLI=target/release/stemma.exe
UI=target/release/stemma-ui.exe
[[ -f "$CLI" ]] || CLI=target/release/stemma
[[ -f "$UI"  ]] || UI=target/release/stemma-ui

if [[ ! -f "$CLI" || ! -f "$UI" ]]; then
    echo "Building (first time takes a minute — the graphics stack)…"
    cargo build --release -p stem_cli -p stem_ui
fi

[[ "$REBUILD" == 1 ]] && rm -rf out

if [[ ! -f out/attested_coastal_modern.ron ]]; then
    mkdir -p out
    echo "Coining a 673-word proto-language…"
    # `asterian_attested` rather than `proto_asterian`: it carries the same phoneme
    # inventory PLUS a stress system, and without prosody the stress-conditioned
    # rules ("final UNSTRESSED vowel loss") can never fire — the engine says so, but
    # the family would be duller for no reason. `new-lexicon` replaces the nine
    # hand-authored words with 673 coined ones.
    "$CLI" new-lexicon fixtures/asterian_attested.ron --seed 42 --out out/proto.ron

    echo "Forking three daughters, each with its own sound-change history…"
    "$CLI" fork out/proto.ron --rules fixtures/rules_coastal.ron \
        --id coastal  --name "Coastal Asterian"  --years 470 --out out/coastal.ron
    "$CLI" fork out/proto.ron --rules fixtures/rules_highland.ron \
        --id highland --name "Highland Asterian" --years 460 --out out/highland.ron
    "$CLI" fork out/proto.ron --rules fixtures/rules_riverine.ron \
        --id riverine --name "Riverine Asterian" --years 420 --out out/riverine.ron

    # Meaning drift is NOT applied to this family, deliberately.
    #
    # `fixtures/drift_coastal.ron` is authored against the attested fixture, where
    # `w_0001` is *takala "star" and holds the sense `sn_star`. In a *generated*
    # lexicon `w_0001` is whatever the concept list puts first ("all"), and no
    # coined word carries a sense at all — so the event would match nothing. The
    # engine says so rather than pretending (`drift.removal_matched_nothing`), and
    # shipping a step whose only output is that note would be theatre.
    #
    # The drift demonstration lives on the fixture it was written for; see
    # docs/GUIDE.md, "Give a word a new meaning".
    echo "Building the meaning-drift example on the fixture it was authored for…"
    "$CLI" fork fixtures/asterian_attested.ron --rules fixtures/rules_coastal.ron \
        --id attested_coastal --name "Coastal (attested)" --years 470 \
        --out out/attested_coastal.ron
    "$CLI" drift out/attested_coastal.ron --drift fixtures/drift_coastal.ron \
        --id attested_coastal_modern --name "Modern Coastal (attested)" --years 30 \
        --out out/attested_coastal_modern.ron
fi

echo
echo "The family, compared:"
"$CLI" cognates out/proto.ron out/coastal.ron out/highland.ron out/riverine.ron \
    --meanings star water sun moon blood tooth 2>/dev/null

if [[ "$OPEN_UI" == 1 ]]; then
    echo
    echo "Opening the explorer. Click a word on the left to see its full history."
    "$UI" out/proto.ron out/coastal.ron out/highland.ron out/riverine.ron
fi
