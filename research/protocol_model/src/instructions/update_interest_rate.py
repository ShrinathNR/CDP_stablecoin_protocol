"""Interest rate update logic - direct port from Rust"""
import time
from typing import Optional, Tuple
from ..state.protocol_config import ProtocolConfig
from ..errors import ArithmeticError
from ..constants import (
    MIN_INTEREST_RATE,
    MAX_INTEREST_RATE,
    PRICE_SCALE,
    INTEREST_SCALE,
    BPS_SCALE,
    YEAR_IN_SECONDS
)

def exponential_approximation(x: int, scale: int) -> int:
    """Port of Rust exponential_approximation
    Using Taylor series of order 3
    All values scaled by INTEREST_SCALE
    """
    # first term: 1
    result = scale

    # second term: x
    result = checked_add(result, x)

    # third term: x^2/2!
    x_squared = checked_mul(x, x) // scale

    result = checked_add(result, x_squared // 2)

    # fourth term: x^3/3!
    x_cubed = checked_mul(x_squared, x) // scale

    result = checked_add(result, x_cubed // 6)

    return result

def calculate_interest_rate(
    stablecoin_price: int,  # i64 in Rust
    base_rate_bps: int,     # u16 in Rust
    sigma_bps: int          # u16 in Rust
) -> int:                   # Returns u128 in Rust
    """Calculate interest rate using exponential approximation"""
    # Calculate price deviation from peg
    price_deviation = PRICE_SCALE - stablecoin_price
    if price_deviation < 0:
        raise ArithmeticError("Price deviation underflow")

    # Calculate exponent
    # First in price scale
    x = checked_mul(price_deviation, BPS_SCALE) // sigma_bps

    # Convert exponent from price scale to interest scale
    x = checked_mul(x, INTEREST_SCALE) // PRICE_SCALE

    # Calculate e^x and multiply by base rate
    exp_term = exponential_approximation(x, INTEREST_SCALE)
    interest_rate = checked_mul(exp_term, base_rate_bps) // BPS_SCALE

    # Clamp to min/max
    return min(max(interest_rate, MIN_INTEREST_RATE), MAX_INTEREST_RATE)

def compound_interest(interest_rate: int, time_elapsed: int) -> int:
    """Port of Rust compound_interest
    All values scaled by INTEREST_SCALE
    """
    # Handle small time periods directly
    if time_elapsed == 0:
        return INTEREST_SCALE
    elif time_elapsed == 1:
        return INTEREST_SCALE + interest_rate
    elif time_elapsed == 2:
        one_plus_rate = INTEREST_SCALE + interest_rate
        return checked_mul(one_plus_rate, one_plus_rate) // INTEREST_SCALE
    elif time_elapsed == 3:
        one_plus_rate = INTEREST_SCALE + interest_rate
        squared = checked_mul(one_plus_rate, one_plus_rate) // INTEREST_SCALE
        return checked_mul(squared, one_plus_rate) // INTEREST_SCALE
    elif time_elapsed == 4:
        one_plus_rate = INTEREST_SCALE + interest_rate
        squared = checked_mul(one_plus_rate, one_plus_rate) // INTEREST_SCALE
        return checked_mul(squared, squared) // INTEREST_SCALE

    # For larger time periods use Taylor expansion
    exp = time_elapsed
    exp_minus_one = time_elapsed - 1
    exp_minus_two = time_elapsed - 2

    base = interest_rate
    base_pow_two = checked_mul(base, base) // INTEREST_SCALE
    base_pow_three = checked_mul(base_pow_two, base) // INTEREST_SCALE

    first_term = INTEREST_SCALE
    second_term = checked_mul(exp, base)
    third_term = checked_mul(checked_mul(exp, exp_minus_one), base_pow_two) // 2
    fourth_term = checked_mul(
        checked_mul(
            checked_mul(exp, exp_minus_one),
            exp_minus_two
        ),
        base_pow_three
    ) // 6

    return checked_add(
        first_term,
        checked_add(
            second_term,
            checked_add(third_term, fourth_term)
        )
    )

def checked_mul(a: int, b: int) -> int:
    """Multiply with overflow checking"""
    result = a * b
    if result > 2**128 - 1:  # u128::MAX
        raise ArithmeticError("Arithmetic overflow in multiplication")
    return result

def checked_div(a: int, b: int) -> int:
    """Divide with overflow checking"""
    if b == 0:
        raise ArithmeticError("Division by zero")
    return a // b

def checked_add(a: int, b: int) -> int:
    """Add with overflow checking"""
    result = a + b
    if result > 2**128 - 1:  # u128::MAX
        raise ArithmeticError("Arithmetic overflow in addition")
    return result

def get_mock_price() -> Tuple[int, int]:
    """Mock function to get price from Pyth
    Returns:
        Tuple[price, exponent] - price with exponent -6 (6 decimal places)
    """
    return (1_000_000, -6)  # $1.00 with 6 decimals

def update_interest_rate(
    protocol_config: ProtocolConfig,
    mock_price: Optional[Tuple[int, int]] = None,
    time_elapsed: Optional[int] = None
) -> None:
    """Port of Rust update_interest_rate"""
    # Calculate time elapsed
    if time_elapsed is None:
        current_timestamp = int(time.time())
        time_elapsed = current_timestamp - protocol_config.last_interest_rate_update
    if time_elapsed == 0:
        return

    # Get current stablecoin price
    price, _ = mock_price if mock_price is not None else get_mock_price()
    
    # Calculate yearly interest rate
    new_interest_rate_yearly = calculate_interest_rate(
        price,
        protocol_config.base_rate,
        protocol_config.sigma
    )

    # Convert to per-second rate and compound
    new_interest_rate = new_interest_rate_yearly // YEAR_IN_SECONDS
    compounded_interest_rate = compound_interest(new_interest_rate, time_elapsed)

    # Update protocol state
    protocol_config.cumulative_interest_rate = checked_mul(
        protocol_config.cumulative_interest_rate,
        compounded_interest_rate
    ) // INTEREST_SCALE
    
    protocol_config.total_debt = checked_mul(
        protocol_config.total_debt,
        compounded_interest_rate
    ) // INTEREST_SCALE
    
    protocol_config.last_interest_rate_update = (
        int(time.time()) if time_elapsed is None 
        else protocol_config.last_interest_rate_update + time_elapsed
    ) 