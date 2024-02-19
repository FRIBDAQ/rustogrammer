# rustogrammer
Histogram data from FRIB analysis pipeline.

Initial usable version.

*  Written in RUST
*  Using parameter items from the output of the FRIB analysis pipeline.

To install (assuming you have the rust build infrastructure installed):

```
cargo build 
```
or if you don't want debugging support but an optimized build:
```
cargo build --release
```
To run the GUI front end you'll need Python3 and the prerequisite packages.
These can be pulled in with PIP via ppackages.bat or you can use whatever your
package manager (if linux) uses to install the packages centrally.

Linux installation:
```
./deploy.sh  {target} {destination}
```
Windows installation:

```
.\install.bat {targat} {destination}
```

Where:

*   {target} is debug if you did not supply the --release flag to cargo or
release if you did.
*   {destination} is the top level installation directory.


On Linux, {destination}/bin will have the rustogrammer binary and a script named gui
to run the gui front end.

On Windows {destination} wll have rustogrammer.exe and gui.bat

You may also want to get the CutiPie visualizer availalbe from 
https://github.com/FRIBDAQ/CutiePie  it allows you to visualize
spectra and create graphical gates.


