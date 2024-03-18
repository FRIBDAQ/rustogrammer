# Tcl REST interface

Two Tcl packages get installed with Rustogramer.  

*  On Windows these are installed in ```%RUSTOGRAMER%\restclients\tcl``` where ```%RUSTOGRAMER%``` is that directory you gaven to the installer.bat file.
*  On Linux these are installed in ```$dest/share/restclients/Tcl```  where ```$dest``` is the install directory you gave install.sh

These are ports of the SpecTcl REST client software and retain those names:

*  SpecTclRESTClient is a package that provides low level REST request/response using the response formats created by both SpecTcl and Rustogramer.
*  SpecTclRestCommand is a package that provides most of the Tcl command extensions that SpecTcl creates, however implemented over the REST interface.  This can be used as a tool to port existing SpecTcl Tcl/Tk based GUIs.

In addition, the program  ```rusttcl.tcl``` in that directory provides a script that you can use to run a Tcl shell that talks to rustogramer (well SpecTcl too for that matter).

## Setting up the packages for use.

There are two ways to setup any Tcl package for use:
*  Defining the environment variable ```TCLLIBPATH``` to include the directory tree that includes the package (e.g. /usr/opt/rustogramer/restclients/tcl on linux).
*  Manually adding the package directory tree to the Tcl ```auto_path``` variable.

Suppose you have a script that defined the variable tclrest_dir to point to the directory that includes the Tcl Rest clients:

```tcl
lappend auto_path $tclrest_dir
```

will add those packages to the package search path used by the ```package require``` command.

How to create an environment variable depends on your environment.  This addition to your .bashrc can add $tclrest_dir to that variable in Linux:

```bash
export TCLIBPATH="$TCLLIBPATH $tclrest_dir
```

In Windows it's probably best to _very carefully_ add this to the registery.  Start regedit and 
navigate to ```HKEY_CURRENT_USER\Environment```  Choose ```Edit->New expandable string value```  Enter the name TCLLIBPATH and the value the path to your the restclients\tcl directory.

## Using The low level client.

Once you are set up to add the package path for the Tcl REST clients to your scripts.  You can use the low leve client.  See the [Tcl REST reference](./chap7_3.md) for reference material.  In this section we're just going to give a brief overview of the package and how to use it.

The Tcl low level client is implemented in an object oriented manner.  You instantiate a client object and then issue it subcommands to get the server to do things.  It's important to note that the client does not attempt to connect with the server until it is asked to interact with it and each interaction, therefore involes a new connection to the server.

Here's a very brief example of how all this works.

```tcl
#    Somewhere above tclrest_dir was defined:

lappend auto_path $tclrest_dir
package rquire SpecTclRESTClient;      # 1



set host locallhost;    # Modify if needed.
set port 8000;          # Modify if needed.         2
set debug 0;            # nonzero to enable debugging output.

set client [SpecTclRestClient %AUTO% -host $host -port $port -debug $debug]; # 3

# Let's see who we're talking to:

set versionInfo [$client version];   # 4

#  Major and minor and editlevel are always present. program_name was added last time:

if {[dict exists $versionInfo program_name]} {;     # 5
    set name [dict get $versionInfo program_name]
} else {
    set name *unknown*
}

set major [dict get $versionInfo major]
set minor [dict get $versionInfo minor]
set edit  [dict get $versionInfo editLevel];     # 6
puts "Connected to $name"
puts "Version $major.$minor-$edit"


```

Refer to the numbered comments above when reading the remarks below

1.  Loads the Rest low level package.
2.  These define the connection parameters for the client.  Note that the client can run in debugging mode, in which case, it output information about the requests it makes and the responses it got.  To run in debug mode set the ```debug``` variable to some non-zero value.
3.  This creates a  client object using our connection parameters.  Note that the first paramter to the instance creation command is the name of a command ensemble that will be created (command ensembles are commands that have subcommands).  The special name ```%AUTO%``` will create a unique command name.  I recommned using ```%AUTO%``` to avoid colliding with other commands.  The name of the command is stored in the ```client``` variable.
4.  This is the first (and  only) interaction with the server.  The ```version``` subcommand requests the server identify itself and its current version number.  The resulting information is stored in the dict ```versionInfo```
5. Originally, when there was only SpecTcl, the only information returned was the major and minor versions and the edit lievel.  When rustogramer's Rest interface was written it, and later SpecTcl added the ```program_name``` key.  This block of code determines if the returned inforation has the ```program_name``` key and, if so, sets the value of ```name``` to it. Otherwise, the value of ```name``` is set to ```*unknown*```
6. The version information is pulled out of the dict and finally everything is printed.
