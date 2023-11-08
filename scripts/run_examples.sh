#!/bin/bash

example_crates=$(cargo metadata --format-version 1 |
    jq -c '.workspace_members' |
    jq -r 'map(select(startswith("examples")) |
                 sub("\\s.*$";"")) | .[]')

for crate in $example_crates; do
    cargo build -p "$crate" --examples
done

for crate in $example_crates; do
    cratedir="${crate#examples-}"
    srcdir="examples/$cratedir/examples"
    example_files=$(find "$srcdir" -type f -name '*.rs' -exec basename {} \; | sed 's/\.[^.]*$//')

    for file in $example_files; do
        cargo run -p "$crate" --example "$file"
    done
done
