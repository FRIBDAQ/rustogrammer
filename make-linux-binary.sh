#!/bin/bash
#  Makee a windows binary distribution.
#  Requirements:
#  -  rust compiler distribution and cargo.
#  -  mdbook for user documentation.
#  cwd is the top level of the rustogramer tree.

# Build the release and debug versions:

cargo build 
cargo build --release
cargo doc --no-deps

# Build the user documentation:

mdbook build docs

# Build a tarball for our stuff:


tar cvzf rustogramer-linux.tar.gz \
      -C .. \
       rustogrammer/target/debug/rustogrammer \
       rustogrammer/target/release/rustogrammer \
       rustogrammer/target/doc \
       rustogrammer/docs/book \
       rustogrammer/restclients \
       rustogrammer/deploy.sh

echo rustogramer-linux.tar.gz - is the linux  binary distribution.
