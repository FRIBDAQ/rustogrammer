#!/bin/bash
##  Deploy rustogrammer:
##    deploy target top-dir
##
#  target - debug or production -- the executables will be
#    in target/$target/rustogrammer
#  top-dir - top deployment directory e.g. /usr/opt/rustogrammer/1.0.0
#
#  The directory tree created will be:
#    top-dir
#       bin   - Where the rustogamer library goes.
#       share - Where the scripty stuff goes:
#          restclients - where the restclient scritps go
#              Python - Python rest client module.
#              Tcl    - Tcl rest client modules.
#


source=$1
dest=$2

if test  "$source" = ""  -o  "$dest" = ""  
then
   echo Usage:
   echo "   "deploy.sh target top-dir
   echo Where:
   echo "   "target is the rust target "(debug or release e.g.)".
   echo "   "dest   is the top level of the installation directory tree.

   exit 1

fi
install -d $dest
install -d $dest/bin
install -d $dest/share

# install the binary:

install target/$source/rustogrammer $dest/bin

# install the client scripts

install -d $dest/share/restclients
install -d $dest/share/restclients/Python
install -d $dest/share/restclients/Tcl

install -m 0644  restclients/Python/* $dest/share/restclients/Python
install -m 0644  restclients/Tcl/*    $dest/share/restclients/Tcl

# Now make a script in bin to run the GUI:

echo "#!/bin/bash" > $dest/bin/gui
echo "(cd $dest/share/restclients/Python; python3 Gui.py" '$@)' >> $dest/bin/gui
chmod 0755 $dest/bin/gui
