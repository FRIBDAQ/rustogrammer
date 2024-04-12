@ECHO off
REM  Installation file for Windows
REM usage: 
REM    install target destination 
REM where
REM     target is a cargo/rust target e.g. debug or release
REM     destination is the top level directory for installation.
REM       The top level directory will get rustogramer.exe - the program
REM       and gui a batch file to run the GUI.
REM       restclients - the rest client tree.

set target=%1
set dest=%2

REM Make sure both target an dest are provided.

IF "%target" == "" GOTO usage
IF "%dest"   == "" GOTO usage

ECHO Target: %target%
ECHO will be installed in %dest%

if exist %dest%\ (
    ECHO deleting existing %dest%
    rmdir %dest% /s
)
mkdir %dest%
mkdir %dest%\restclients
mkdir %dest%\restclients\python
mkdir %dest%\restclients\tcl
mkdir %dest%\docs
mkdir %dest%\docs\user
mkdir %dest%\docs\internal

REM Install rustogramer.exe:

COPY target\%target%\rustogrammer.exe %dest%

REM Install the restclients:

COPY restclients\tcl\* %dest%\restclients\tcl
COPY restclients\python\* %dest%\restclients\python

REM Now the batchfile to run the Python GUI

ECHO CD %dest%\restclients\python  >%dest%\GUI.bat
ECHO PYTHON Gui.py %%* >>%dest%\GUI.bat

REM Documentation:

XCOPY /E/Q target\doc\ %dest%\docs\internal\
XCOPY /E/Q docs\book\ %dest%\docs\user\


ECHO %dest%\rustogrammer will now run the histogramer.
ECHO %dest%\GUI   will now run the Python GUI
ECHO Point a web browser at:
ECHO %dest%\docs\user\index.html - for user Documentation
ECHO %dest%\docs\internal\rustogramer\index.html - For internals documentation. 
ECHO If you have installed CutiePie you can use it as a visualizer
ECHO for you spectra.


exit /batch
:usage
    ECHO Usage:
    ECHO    install target destination
    ECHO Where:
    ECHO   target - the Rust target of rustogramer built by Cargo 
    ECHO            e.g. debug"
    ECHO   destination - Top level installation directory
exit /batch