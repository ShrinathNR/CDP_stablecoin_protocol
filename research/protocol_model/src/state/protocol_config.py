"""Protocol configuration and state management"""
import time
from dataclasses import dataclass
from ..constants import INTEREST_SCALE

@dataclass
class ProtocolConfig:
    """Represents the protocol's configuration and state, matching Rust exactly"""
    stable_mint: str  # Using string instead of Pubkey
    protocol_fee: int
    redemption_fee: int
    mint_fee: int
    base_rate: int
    sigma: int
    auth_bump: int = 0  # Simulated in Python
    bump: int = 0       # Simulated in Python
    cumulative_interest_rate: int = INTEREST_SCALE  # Start at 1.0 scaled
    last_interest_rate_update: int = 0  # Will be set during initialization
    stablecoin_price_feed: str = ""
    total_debt: int = 0
    stake_points: int = 0

    INITIAL_CUMULATIVE_RATE: int = INTEREST_SCALE

    def calculate_current_debt(self, position_debt: int, position_initial_index: int) -> int:
        """Calculate the current debt for a position accounting for accrued interest"""
        # current_debt = position_debt * current_index / initial_index
        return (position_debt * self.cumulative_interest_rate) // position_initial_index

    def update_totals(self, debt_change: int) -> None:
        """Update protocol totals when debt changes"""
        if debt_change > 0:
            self.total_debt += debt_change
        else:
            self.total_debt -= abs(debt_change) 