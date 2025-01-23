"""Custom errors for the protocol simulation"""

class ProtocolError(Exception):
    """Base error class for protocol errors"""
    pass

class ArithmeticError(ProtocolError):
    """Error for arithmetic overflow/underflow"""
    pass

class InvalidPriceError(ProtocolError):
    """Error for invalid or stale price data"""
    pass

class InvalidCollateralError(ProtocolError):
    """Error for invalid collateral operations"""
    pass

class InvalidPositionError(ProtocolError):
    """Error for invalid position operations"""
    pass

class InsufficientCollateralError(ProtocolError):
    """Error for insufficient collateral"""
    pass

class ExcessiveDebtError(ProtocolError):
    """Error for excessive debt"""
    pass 