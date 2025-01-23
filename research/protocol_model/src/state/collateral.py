"""Collateral vault state management"""
from dataclasses import dataclass
from ..constants import (
    DEFAULT_MAX_LTV,
    DEFAULT_LIQ_THRESHOLD,
    DEFAULT_LIQ_PENALTY,
    BPS_SCALE,
    INTEREST_SCALE
)

@dataclass
class CollateralVault:
    """Represents a collateral type vault"""
    mint: str  # Using string instead of Pubkey
    price_feed: str
    total_deposited: int = 0
    max_ltv: int = DEFAULT_MAX_LTV
    liquidation_threshold: int = DEFAULT_LIQ_THRESHOLD
    liquidation_penalty: int = DEFAULT_LIQ_PENALTY
    
    def deposit(self, amount: int) -> None:
        """Deposit collateral"""
        self.total_deposited += amount
        
    def withdraw(self, amount: int) -> None:
        """Withdraw collateral"""
        if amount > self.total_deposited:
            raise ValueError("Insufficient collateral in vault")
        self.total_deposited -= amount
        
    def get_max_debt(self, collateral_amount: int, price: int) -> int:
        """Calculate maximum debt allowed for given collateral amount"""
        # max_debt = collateral_amount * price * max_ltv / BPS_SCALE
        return (collateral_amount * price * self.max_ltv) // BPS_SCALE
        
    def is_liquidatable(self, collateral_amount: int, debt_amount: int, price: int) -> bool:
        """Check if position is liquidatable"""
        if debt_amount == 0:
            return False
            
        # current_ltv = debt_amount * BPS_SCALE / (collateral_amount * price)
        current_ltv = (debt_amount * BPS_SCALE) // (collateral_amount * price)
        return current_ltv > self.liquidation_threshold 