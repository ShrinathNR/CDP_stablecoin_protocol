"""Position state management"""
from dataclasses import dataclass
from ..constants import INTEREST_SCALE

@dataclass
class Position:
    """Represents a CDP position - direct port from Rust"""
    user: str  # Using string instead of Pubkey
    collateral_amount: int  # u64 in Rust
    debt_amount: int  # u64 in Rust
    prev_cumulative_interest_rate: int  # u128 in Rust
    initial_interest_index: int = INTEREST_SCALE
    liquidation_ltv: int = 8000  # 80% in bps
    origination_fee: int = 0
    
    def is_healthy(self):
        """Check if position is healthy (not liquidatable)"""
        # This would need actual price data in practice
        return True  # Mocked for now
        
    def update_collateral(self, amount_change):
        """Update position collateral"""
        if amount_change > 0:
            self.collateral_amount += amount_change
        else:
            if self.collateral_amount < abs(amount_change):
                raise ValueError("Insufficient collateral")
            self.collateral_amount -= abs(amount_change)
            
    def update_debt(self, amount_change, current_interest_index):
        """Update position debt"""
        # First update debt to current value based on interest accrued
        self.debt_amount = (self.debt_amount * current_interest_index) // self.initial_interest_index
        self.initial_interest_index = current_interest_index
        
        # Then apply the change
        if amount_change > 0:
            self.debt_amount += amount_change
        else:
            if self.debt_amount < abs(amount_change):
                raise ValueError("Insufficient debt")
            self.debt_amount -= abs(amount_change) 