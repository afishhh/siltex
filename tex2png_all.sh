#!/usr/bin/env bash

if [[ $# -ne 2 ]]; then
  echo "syntax: $0 <input directory> <output directory>" >&2
  exit 1
fi

shopt -s globstar

in="$1"
out="$2"
for f in "$in"/**/*.tex; do
  noprefix=${f##"$in"}
  mkdir -p "$(dirname "$out/$noprefix")"; ./target/release/siltex tex2png -o "$out/${noprefix%%.tex}.png" "$f" || echo "$f failed";
done
