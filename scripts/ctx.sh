#!/usr/bin/env bash
# Bundle files into one text file to attach to a chat:  ./scripts/ctx.sh CLAUDE.md src/app.rs
cd "$(dirname "$0")/.."
out=context.txt
: > "$out"
for f in "$@"; do
  printf '\n===== %s =====\n' "$f" >> "$out"
  cat "$f" >> "$out"
done
echo "wrote $out ($(wc -c < "$out") bytes)"
