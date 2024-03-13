# Chapter 1 Introduction

## What is Rustogramer

Rustogramer is a histograming application written in the Rust programming language.
The Rust programing language is a language that is optimized for reliable programing.
By this I mean:

*   It is very hard to generate memory leaks in Rust programs.
*   Threading is layered on the language in a way that  makes many of the issues normally associated with threading (race conditions, deadlocks) very difficult.

The support for reliable programing in Rust makes it a good choice for mission critical software at the FRIB.

Rustogramer is written as a closed program.  Unlike NSCLSpecTcl, for example,  you don't have to and are not allowed to write user code sections to make a version of Rustogramer suitable for use.
This is good because:

* Most SpecTcl ``bugs'' are actually errors in user code.
* You won't have to learn Rust to use Rustogramer.

Other interesting features of Rustogramer:

* It is portable between Linux and Windows.  It may well run on OS-X but we don't test/debug on that system so no assurances.  With the FRIB standard desktop a Windows system, this means you can extend you data analysis to the power of your desktop system.
* It is noninteractive.  This allows you to run it in the background, only interacting with it as desired.
* It is compatible with the CutiePie visualizer see [FRIB docs on CutiePie](https://docs.nscl.msu.edu/daq/newsite/qtpy/index.html)
* It provides a REST-like interface that allows you to interact with it either through the provided Python GUI or with any GUI you might write in Python or Tcl.  Since REST protocols are inherently stateless, you can start or stop any number of GUIs at any time. 




