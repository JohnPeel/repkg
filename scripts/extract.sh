#!/bin/bash

PSYCHONAUTS_PATH=~/psychonauts/
OUTPUT=output

#find $PSYCHONAUTS_PATH -name '*.pkg' -print0 |
#    while IFS= read -r -d '' FILE; do
#        FILE_NAME=$(basename $FILE)
#        cargo run -q --manifest-path=../Cargo.toml --bin=repkg -- extract-pkg "$FILE" --output "$OUTPUT"
#    done

find $PSYCHONAUTS_PATH -name '*.ppf' -print0 |
    while IFS= read -r -d '' FILE; do
        FILE_NAME=$(basename $FILE)
        cargo run -q --manifest-path=../Cargo.toml --bin=repkg -- extract-ppf "$FILE" --output "$OUTPUT"
    done

#find $PSYCHONAUTS_PATH -name '*.apf' -print0 |
#    while IFS= read -r -d '' FILE; do
#        FILE_NAME=$(basename $FILE)
#        cargo run -q --manifest-path=../Cargo.toml --bin=repkg -- extract-apf "$FILE" --output "$OUTPUT"
#    done
