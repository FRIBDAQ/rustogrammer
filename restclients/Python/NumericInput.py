''' This module contains Comboboxes and validators for 
    Different types of numeric inputs.  The comboboxes are editable with the
    idea that frequently used values will therefore be added to the
    combobox and be selectable rather than needing to be typed in.

'''

from PyQt5.QtWidgets import QComboBox
from PyQt5.QtGui import  QValidator

''' Float Validator - This validator returns (see limits below):
    -  Acceptable for any integer.
    -  Intermediate for any number trailed with a '.'
    -  Intermediate for any string that's only whitespace.
    -  Acceptable for any valid floating point number.
    -  Invalid for anything else.

    - Optional, inclusive lower and upper limits are supported via the properties  
        * lowLimit
        * upperLimit
      if a limit is 'None' no limit is enforced for that direction.  This allows
      for semiopen intervals.

    If there's a low limit but the value is less than it it validats as
    Intermediate.  If a high limit and the value is greater than it it's
    Invalid    

'''

class FloatValidator(QValidator):
    def __init__(self, *args):
        super().__init__(*args)
        self.lowLimit = None
        self.upperLimit = None

    ''' Support for the properties: '''
    def lowLimit(self): 
        return self.lowLimit
    def setLowLimit(self, value) :
        self.lowLimit = value        
    def upperLimit(self):
        return self.upperLimit
    def setUpperLimit(self, value) :
        self.upperLimit = value
    ''' Implement validation which is, surprisingly, tricky. '''

    def checkLimits(self, s):
        f = float(s)
        if self.lowLimit is not None and f < self.lowLimit:
            return QValidator.Intermediate
        if self.upperLimit is not None and f > self.upperLimit:
            return QValidator.Invalid
        return QValidator.Acceptable

    def validate(self, s, pos):
        if s.isspace():
            return QValidator.Intermediate
        try:
            int(s)
            return self.checkLimits(s)
        except:
            pass
        try:
            float(s)
            return self.checkLimits(s)
        except:
            pass
        # Last case an integer followed by a .
        if len(s) >= 2:
            if s[-1] == '.':
                try:
                    int(s[0:-2])
                    return QValidator.Intermediate
                except:
                    return QValidator.Invalid
        # Maybe I relent and believe everything could be edited to good
        return QValidator.Intermediate

''' A integer validator is the same as a float validator but floating point
    values are intermediate not acceptable.
'''
class IntegerValidator(FloatValidator):
    def __init__(self, *args):
        super().__init__(*args)

    def validate(self, s, pos):
        if s.isspace():
            return QValidator.Intermediate
        try:
            int(s)
            return self.checkLimits(s)
        except:
            pass
        # Maybe I relent and believe everything could be edited to good
        return QValidator.Intermediate

''' Validator for unsigned integer is just an integer validator
    with low limit set to 0
'''
class UnsignedValidator(IntegerValidator):
    def __init__(self, *args):
        super().__init__(*args)
        self.setLowLimit(0)


    