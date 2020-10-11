#!/bin/sh

set -x

make $COMMAND

if [ "$COV" = "yes" ]
then
    cargo install cargo-tarpaulin
    cargo tarpaulin -v --ignore-tests --lib --ciserver travis-ci \
        --coveralls "$TRAVIS_JOB_ID"
fi
