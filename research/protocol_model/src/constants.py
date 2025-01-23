# Fixed point scale factors
YEAR_IN_SECONDS = 365 * 24 * 60 * 60  # Seconds in a year
PRICE_SCALE = 1_000_000  # 6 decimals for price
BPS_SCALE = 10_000  # Basis points (100% = 10000)
INTEREST_SCALE = 1_000_000_000_000_000_000  # 1e18 for interest
MIN_INTEREST_RATE = INTEREST_SCALE // 100  # 1% APR
MAX_INTEREST_RATE = INTEREST_SCALE * 30 // 100  # 30% APR

# Position constants
MAX_LTV = 8000  # 80% in basis points

# LTV constants
DEFAULT_MAX_LTV = 8000           # 80% in bps
DEFAULT_LIQ_THRESHOLD = 8500     # 85% in bps
DEFAULT_LIQ_PENALTY = 500        # 5% in bps

# Time constants
YEAR_IN_SECONDS = 365 * 24 * 60 * 60  # 365 days * 24 hours * 60 minutes * 60 seconds

# Fee constants
DEFAULT_PROTOCOL_FEE = 0         # 0% in bps
DEFAULT_REDEMPTION_FEE = 50      # 0.5% in bps
DEFAULT_MINT_FEE = 0             # 0% in bps 