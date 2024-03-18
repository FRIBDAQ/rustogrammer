# Chapter 5 - Using CutiePie to display histograms

The [Cutie Pie Displayer](http://github.com/FRIBDAQ/CutiePie) uses the  spectrum shared memory mirroring capability of SpecTcl and Rustogramer as well as the REST interface to allow you to display and interact with the histograms produced by both programs.  CutiePie can run in both Linux and Windows, bringing, along with rustogramer,  fully functional histograming to the Windows desktop.

At the FRIB, the standalon CutiePie will normally be installed in ```/usr/opt/cutiepie/```*x.y-nnn* where x.y.nnn is the version of CutiePie.  To run Cutiepie assuming you have defined the environment variable CUTIETOP to point at this directory:

```
$CUTIETOP/bin/CutiePie
```

In Windows, typically CutiePie is installed in ```C:\CutiePie``` and you can start it via 

```
\CutiePie\bin\cutiepie
```


Naturally, in windows, you can make a desktop shortcut.  

When Cutiepie is running,  Use it's ```Connect``` button to connect it to SpecTcl or Rustogramer.
Note that due to differences in how shared memory and mirroring works between Linux and Windows, you cannot mix environments.  You can only:

1.  Run native Windows CutiePie with native Windows Rustogramer.
2.  Run Linux/WSL CutiePie with Linux/WSL Rustogramer or SpecTcl.

For CutiePie user documentation see <a href='https://docs.nscl.msu.edu/daq/newsite/qtpy/index.html' target='_blank'>The FRIB documentation page on CutiePie</a>



