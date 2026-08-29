#!/usr/bin/env bash
# Build .tex from docs/latex/ -> aux + PDFs in docs/build/ (ignored).
# CI publishes copies to docs/pdfs/.
#   ./docs/build.sh               # build all
#   ./docs/build.sh design-draft  # build one

set -e
cd "$(dirname "$0")/latex"
mkdir -p ../build

if ! command -v pdflatex &>/dev/null; then
  echo "pdflatex not found. Install MacTeX or TeX Live." >&2
  exit 1
fi

if [[ -n "${1:-}" ]]; then
  f="${1%.tex}.tex"
  [[ -f "$f" ]] || { echo "No such file: $f" >&2; exit 1; }
  set -- "$f"
else
  set -- *.tex
  [[ -f "$1" ]] || { echo "No .tex files in $(pwd)" >&2; exit 1; }
fi

for f in "$@"; do
  echo "Building $f ..."
  pdflatex -output-directory=../build "$f"
  pdflatex -output-directory=../build "$f"
done

echo "PDFs in docs/build/ — CI publishes to docs/pdfs/"
