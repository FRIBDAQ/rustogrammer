REM  Makee a windows binary distribution.
REM  Requirements:
REM  -  rust compiler distribution and cargo.
REM  -  mdbook for user documentation.
REM  cwd is the top level of the rustogramer tree.

REM Build the release and debug versions:

cargo build 
cargo build --release
cargo doc --no-deps

REM Build the user documentation:

mdbook build docs

REM Build a tarball for our stuff:


tar cvzf rustogramer-windows.tar.gz ^
      -C .. ^
       rustogrammer/target/debug/rustogrammer.exe ^
       rustogrammer/target/release/rustogrammer.exe ^
       rustogrammer/target/doc ^
       rustogrammer/docs/book ^
       rustogrammer/restclients ^
       rustogrammer/install.bat

ECHO rustogramer-windows.tar.gz - is the windows binary distribution.