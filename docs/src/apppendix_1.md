# Appendix I - Installing Rustogramer
 

 Prior to version 1.1.1, you could only install Rustogramer from sources.  We will document that procedure, in case you want to do that as that option is still available.

 With 1.1.1 and later a binary distribution was also made available.

 ## Installation from source

 Installation from source requires the following:

 *  The Rust compilation environment.  The [Getting Started page](https://www.rust-lang.org/learn/get-started) of the Rust website descdribes how to do this and is our recommended way to get this.
 *  mdbook to build this user documentation.  Once the Rust compliation environment is installed you can install that using the command ```cargo install mdbook```


### Getting and building the program

 When you install from sources, you will need to download a release source from [the rustogramer git repository](https://github.com/FRIBDAQ/rustogrammer/releases).  If on windows, you should grab the .zip for the source code and if linux the .tar.gz.

After you have unwrapped the source code and set your working directory to the top level of the unwrapped distribution, you can build the debug and release versions of Rustogramer and user documentation using the same commands on windows and linux:

```bash
cargo build
cargo build --release
mdbook build docs
```
 
 ## Doing the installation:

 On linux you can run the shell script ```deploy.sh``` to install the package in some directory tree. On windows, you can use ```install.bat``` to do the same.  Both scripts require the same two command line parameters:  The version of Rustogramer (dev or release) you want installed and the destination directory.


#### Final installation on Windows.

 For example, on windows you might:

 ```cmd
.\install.bat release \rustogramer
```

to install the release version of rustogramer.  At the bottom of the output you'll get:

```
\rustogramer\rustogrammer will now run the histogramer.
\rustogramer\GUI   will now run the Python GUI
Point a web browser at:
\rustogramer\docs\user\index.html - for user Documentation      
\rustogramer\docs\internal\rustogramer\index.html - For internals documentation.
If you have installed CutiePie you can use it as a visualizer   
for you spectra.
```

If you want to install the debug version you can use e.g.:

```cmd
.\install.bat debug \rustogramer
```

#### Final installation on Linux

For example on Linux you might:

```bash
./deploy.sh production /usr/opt/rustogramer/1.1.1
```

Or again to install the debug version:

```bash
./deply.sh debug /usr/opt/rustogramer/1.1.1
```

## Installing from binaries.

Beginning with release 1.1.1, the release products include files named:
* rustogramer-linux.tar.gz - Binary distribution for Linux
* rustogramer-widows.zip  - Binary distribution for Windows.

These are made with the scripts

```
make-linux-binary.sh
```
and

```
make-windows-binary.bat
```

in the source distribution.

To install from binaries, grab the distribution approprate to your system and follow the instructions in [Doing the installation](#doing-the-installation) above.