## Format of data Rustogramer accepts

Data files Rustogramer can analyze contain NSCLDAQ ring items.  The set of ring items has been extended, however and Rustogramer only pays attention to a very few of the items.  The items it cares about are defined in the ```AnalysisRingItem.h``` header file.  

Note that analysis ring items doin't have body headers. Therefore if you want to retain the timestamp in your processed data, you must allocate a parameter to it and pull it out of the raw event body headers.

Therefore a Ring item header for analysis Ring item is defined as:

```c++
#pragma pack(push, 1)
namespace frib { namespace analysis {
typedef struct _RingItemHeader {
        std::uint32_t s_size;
        std::uint32_t s_type;
        std::uint32_t s_unused;    // must be sizeof(std::uint32_t).
    } RingItemHeader, *pRingItemHeader;
}}
#pragma pack(pop)
```
Where, as in NSCLDAQ ring items, ```s_size``` is the size in bytes of the entire ring item (including the ```s_size``` field itself).  The ```s_type``` field is the type of the ring item, where 
```AnalysisRingItems.h``` defines several new ring item types in the user ring item type domain.
Note as well that RingItemHeader is, like all definitions for the FRIB analysis pipeline framework in the ```frib::analysis``` namespace and the two ```#pragma``` statements are g++ magic to ensure that the structures are tightly packed.

### Parameter definition ring items

Each parameter file opens with a ```ParameterDefinitions``` ring item.  Parameter definitions associate an integer with the names of each parameter written to the file:

```c++
#pragma pack(push, 1)
namespace frib { namespace analysis {
typedef struct _ParameterDefintion {
    std::uint32_t s_parameterNumber;
    char          s_parameterName[0];   // Actually a cz string.
} ParameterDefinition, *pParameterDefinition;

/**
    *  parameter defintion ring item
    *  sizeof  is not useful.
    */
typedef struct  _ParameterDefinitions {
    RingItemHeader s_header;
    std::uint32_t  s_numParameters;
    ParameterDefinition s_parameters [0];
} ParameterDefinitions, *pParameterDefinitions;

}}
#pragma pack(pop)
```

Starting with the defiition of ```ParameterDefinitions```;  This ring item has the normal ring item header.  ```s_numParameters``` is the number of ```ParameterDefinition``` items that follow.  Each ```ParameterDefinition``` provides a parameter number; ```s_parameterNumber``` and a null terminated parameter name string who's first character is at ```s_parameterName```.  This implies that the ```ParameterDefinition``` struct is really variable length.

The ```s_type``` field of the ```RingItemHeader``` for parameter definition ring items will be
```frib::analysis::PARAMETER_DEFINITIONS```.  At the time this is being written, this is ```32768```, however you should always relay on the symbolic name as that insulates your source code from changes.

A ```ParameterDefinitions``` ring item prior to actual event data is mandatory.

### Variable Value ring items.

Our description of the FRIB analysis pipeline did not mention many of its capabilities.   One capability is the ability to define values in the parameter definition file that can steer the computations of your workers.  For example, you might provide calibration values to compute some calibrated parameters fromt he raw event parameters.

The NSCLSpecTcl ```CTreeVariable```  and ```CTreeVariableArray``` classes have identical classes (in the ```frib::analysis``` namespace) which bind objects to the names of treevariables and treevariablearrays you define in your configuration file.

The ```frib::analysis::VariableItem``` ring item documents the values of these variables at the time the parameter file was produced.  The ```VariableItem``` ring item will immediately follow the ```ParmeterDefinitions``` ring item and is optional.

```c++
#pragma pack(push, 1)
namespace frib { namespace analysis {
typedef struct _Variable {
    double s_value;
    char   s_variableUnits[MAX_UNITS_LENGTH];     // Fixed length
    char   s_variableName[0];       // variable length
} Variable, *pVariable;

typedef struct _VariableItem {
    RingItemHeader s_header;
    std::uint32_t  s_numVars;
    Variable       s_variables[0];
    
} VariableItem, *pVariableItem;

}}
#pragma pack(pop)
```

The ```s_header.s_type``` value for these ring items is ```frib::analysis::VARIABLE_VALUES```.  As you might expect, ```s_numVars``` is the number of variable definitions that follow starting at ```s_variables```

A variable has a name, a value and units of  measure.  These are held in a variable length ```frib::analysis::Variable``` struct:
*  ```s_value``` is the value of the variable.  For maximum flexibility, these are represented in double precision floats.
*  ```s_variableUnits``` - is  a character array ```frib::analysis::MAX_UNITS_LENGTH``` long that contains the variable's units of measure.  The string itself is null terminated and no assurances are made about the values following that null.
*  ```s_variableName``` - is the first character of the variable's name.  The name is stored as a null terminated string and is what makes the ```Variable``` struct variable length.

### Parameter data ring items.

```frib::analysis::ParameterItem``` ring items hold the paramters in one event extracted by your worker from the raw data.  In general, the remainder of the parameter file will be composed of these items as well as items from the original event file that the pipeline ignored.

Here is the structure of ```frib::analysis::ParamterItem``` ring items:

```c++
#pragma pack(push, 1)
namespace frib { namespace analysis {
typedef struct _ParameterValue {
    std::uint32_t s_number;
    double        s_value;
} ParameterValue, *pParameterValue;

/*
    * Ring item of parameter unpacked data.
    * sizeof is worthless.
    */
typedef struct _ParameterItem {
    RingItemHeader s_header;
    std::uint64_t  s_triggerCount;
    std::uint32_t  s_parameterCount;
    ParameterValue s_parameters[0];
} ParameterItem, *pParameterItem;
}
#pragma pack(pop) 
```

The ```s_header.s_type``` value of this type of ring item will be ```frib::analysis::PARAMETER_DATA```.
*  ```s_triggerCount``` - is the number of the event.  Each raw event is assigned a trigger number by the pipeline dealer.  That trigger number is just a sequential value and is used to re-sort the data emitted by the workers into the original event order.   In this way the framework can be used wih data that are not timestamped.
*   ```s_parameterCount``` - The number of parameters unpacked from this event.  A parameter that was not assigned a value by the worker's unpacker for this event will not have a parameter value in this ring item.
*   ```s_parameters``` is an array of ```s_parameterCount``` instances of ```frib::analysis::ParameterValue``` objects.

Parameter values contain:
*   ```s_number``` the parameter number of a parameter in the ```ParameterValue``` ring item.  This number is used to make a correspondence between parameter names and actual parameters.
*   ```s_value``` for this event, the value of that parameter.


Suppose, for example, that ther ewas a ```ParameterDefinition``` with ```s_parameterNumber``` having a value of ```124``` and a ```s_paramterName``` of ```"dummy"```;  Any ```ParameterValue``` with ```s_number == 124``` will have ```s_value``` set to the value of parameter ```dummy``` for this event.

