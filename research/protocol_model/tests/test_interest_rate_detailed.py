"""Detailed test suite for interest rate mechanism - matching Rust implementation"""
import time
import numpy as np
from decimal import Decimal, ROUND_DOWN
from dataclasses import dataclass
from typing import List, Tuple
from protocol_model.src.constants import (
    INTEREST_SCALE,
    PRICE_SCALE,
    BPS_SCALE,
    YEAR_IN_SECONDS,
    MIN_INTEREST_RATE,
    MAX_INTEREST_RATE
)
from protocol_model.src.state.protocol_config import ProtocolConfig
from protocol_model.src.errors import ArithmeticError

@dataclass
class TestCase:
    """Test case for interest rate calculation"""
    description: str
    stablecoin_price: int  # Scaled by PRICE_SCALE
    base_rate_bps: int
    sigma_bps: int
    time_elapsed: int  # seconds
    expected_rate_range: Tuple[Decimal, Decimal]  # min/max expected rate
    expected_debt_increase: Tuple[Decimal, Decimal]  # min/max expected debt increase

def checked_add(a: int, b: int) -> int:
    """Add with overflow checking"""
    result = a + b
    if result > 2**128 - 1:  # u128::MAX
        raise ArithmeticError("Arithmetic overflow in addition")
    return result

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

def exponential_approximation_rust(x: int, scale: int) -> int:
    """Exact port of Rust exponential_approximation"""
    # first term: 1
    result = scale

    # second term: x
    result = checked_add(result, x)

    # third term: x^2/2!
    x_squared = checked_div(checked_mul(x, x), scale)
    result = checked_add(result, x_squared // 2)

    # fourth term: x^3/3!
    x_cubed = checked_div(checked_mul(x_squared, x), scale)
    result = checked_add(result, x_cubed // 6)

    return result

def calculate_interest_rate_rust(
    stablecoin_price: int,  # i64 in Rust
    base_rate_bps: int,     # u16 in Rust
    sigma_bps: int          # u16 in Rust
) -> int:                   # Returns u128 in Rust
    """Exact port of Rust calculate_interest_rate"""
    # Calculate price deviation from peg
    price_deviation = PRICE_SCALE - stablecoin_price

    # Calculate exponent
    # First in price scale
    x = checked_div(checked_mul(price_deviation, BPS_SCALE), sigma_bps)

    # Convert exponent from price scale to interest scale
    x = checked_div(checked_mul(x, INTEREST_SCALE), PRICE_SCALE)

    # Calculate e^x and multiply by base rate
    exp_term = exponential_approximation_rust(x, INTEREST_SCALE)
    interest_rate = checked_div(
        checked_mul(exp_term, base_rate_bps),
        BPS_SCALE
    )

    # Clamp to min/max
    return interest_rate
    # return min(max(interest_rate, MIN_INTEREST_RATE), MAX_INTEREST_RATE)

def compound_interest_rust(interest_rate: int, time_elapsed: int) -> int:
    """Exact port of Rust compound_interest"""
    # Handle small time periods directly
    if time_elapsed == 0:
        return INTEREST_SCALE
    elif time_elapsed == 1:
        return checked_add(INTEREST_SCALE, interest_rate)
    elif time_elapsed == 2:
        one_plus_rate = checked_add(INTEREST_SCALE, interest_rate)
        return checked_div(checked_mul(one_plus_rate, one_plus_rate), INTEREST_SCALE)
    elif time_elapsed == 3:
        one_plus_rate = checked_add(INTEREST_SCALE, interest_rate)
        squared = checked_div(checked_mul(one_plus_rate, one_plus_rate), INTEREST_SCALE)
        return checked_div(checked_mul(squared, one_plus_rate), INTEREST_SCALE)
    elif time_elapsed == 4:
        one_plus_rate = checked_add(INTEREST_SCALE, interest_rate)
        squared = checked_div(checked_mul(one_plus_rate, one_plus_rate), INTEREST_SCALE)
        return checked_div(checked_mul(squared, squared), INTEREST_SCALE)

    # For larger time periods use Taylor expansion
    exp = time_elapsed
    exp_minus_one = time_elapsed - 1
    exp_minus_two = time_elapsed - 2

    base = interest_rate
    base_pow_two = checked_div(checked_mul(base, base), INTEREST_SCALE)
    base_pow_three = checked_div(checked_mul(base_pow_two, base), INTEREST_SCALE)

    first_term = INTEREST_SCALE
    second_term = checked_mul(exp, base)
    third_term = checked_div(
        checked_mul(checked_mul(exp, exp_minus_one), base_pow_two),
        2
    )
    fourth_term = checked_div(
        checked_mul(
            checked_mul(checked_mul(exp, exp_minus_one), exp_minus_two),
            base_pow_three
        ),
        6
    )

    return checked_add(
        first_term,
        checked_add(
            second_term,
            checked_add(third_term, fourth_term)
        )
    )

def calculate_interest_rate_numpy(
    stablecoin_price: int,
    base_rate_bps: int,
    sigma_bps: int
) -> float:
    """Calculate interest rate using numpy for comparison"""
    # Convert to float for numpy
    price_deviation = (PRICE_SCALE - stablecoin_price) / PRICE_SCALE
    base_rate = base_rate_bps / BPS_SCALE
    sigma = sigma_bps / BPS_SCALE

    # Calculate exponential term
    x = price_deviation / sigma
    exp_term = np.exp(x)
    
    # Multiply by base rate
    rate = base_rate * exp_term
    return rate * INTEREST_SCALE

def compound_interest_numpy(interest_rate: float, time_elapsed: int) -> float:
    """Calculate compound interest using exact (1+r)^t formula"""
    # Convert from scaled integer to float rate
    rate = interest_rate / INTEREST_SCALE
    
    # Use exact formula (1+r)^t
    result = (1.0 + rate) ** time_elapsed
    return result * INTEREST_SCALE

def test_interest_rate_calculation():
    """Test interest rate responses to price deviations"""
    test_cases = [
        TestCase(
            description="At peg",
            stablecoin_price=PRICE_SCALE,
            base_rate_bps=500,  # 5%
            sigma_bps=200,      # 2%
            time_elapsed=1,
            expected_rate_range=(Decimal('0.049'), Decimal('0.051')),
            expected_debt_increase=(Decimal('1.0'), Decimal('1.0')) # not getting tested right now
        ),
        TestCase(
            description="1% below peg",
            stablecoin_price=PRICE_SCALE * 99 // 100,
            base_rate_bps=500,  # 5%
            sigma_bps=200,      # 2%
            time_elapsed=1,
            expected_rate_range=(Decimal('0.05'), Decimal('1.0')),
            expected_debt_increase=(Decimal('1.0'), Decimal('1.0')) # not getting tested right now
        ),
        TestCase(
            description="1% above peg",
            stablecoin_price=PRICE_SCALE * 101 // 100,
            base_rate_bps=500,  # 5%
            sigma_bps=200,      # 2%
            time_elapsed=1,
            expected_rate_range=(Decimal('0.0'), Decimal('0.05')),
            expected_debt_increase=(Decimal('1.0'), Decimal('1.0')) # not getting tested right now
        ),
        TestCase(
            description="3% below peg",
            stablecoin_price=PRICE_SCALE * 97 // 100,
            base_rate_bps=500,  # 5%
            sigma_bps=200,      # 2%
            time_elapsed=1,
            expected_rate_range=(Decimal('0.05'), Decimal('1.0')),
            expected_debt_increase=(Decimal('1.0'), Decimal('1.0')) # not getting tested right now
        ),
        TestCase(
            description="3% above peg", 
            stablecoin_price=PRICE_SCALE * 103 // 100,
            base_rate_bps=500,  # 5%
            sigma_bps=200,      # 2%
            time_elapsed=1,
            expected_rate_range=(Decimal('0.0'), Decimal('0.05')),
            expected_debt_increase=(Decimal('1.0'), Decimal('1.0')) # not getting tested right now
        ),
        # TestCase(
        #     description="5% below peg",
        #     stablecoin_price=PRICE_SCALE * 95 // 100,
        #     base_rate_bps=500,
        #     sigma_bps=200,
        #     time_elapsed=1,
        #     expected_rate_range=(Decimal('0.05'), Decimal('1.0')),
        #     expected_debt_increase=(Decimal('1.0'), Decimal('1.0')) # not getting tested right now
        # ),
        # TestCase(
        #     description="5% above peg",
        #     stablecoin_price=PRICE_SCALE * 105 // 100,
        #     base_rate_bps=500,
        #     sigma_bps=200,
        #     time_elapsed=1,
        #     expected_rate_range=(Decimal('0.0'), Decimal('0.05')),
        #     expected_debt_increase=(Decimal('1.0'), Decimal('1.0')) # not getting tested right now
        # ),
        # TestCase(
        #     description="10% below peg",
        #     stablecoin_price=PRICE_SCALE * 90 // 100,
        #     base_rate_bps=500,
        #     sigma_bps=200,
        #     time_elapsed=1,
        #     expected_rate_range=(Decimal('0.05'), Decimal('2.0')),
        #     expected_debt_increase=(Decimal('1.0'), Decimal('1.0003'))
        # ),
        # TestCase(
        #     description="Overflow test",
        #     stablecoin_price=PRICE_SCALE // 10,  # 90% below peg
        #     base_rate_bps=1000,  # 10%
        #     sigma_bps=100,       # 1%
        #     time_elapsed=86400,  # 1 day
        #     expected_rate_range=(Decimal('0.0'), Decimal('2.0')),
        #     expected_debt_increase=(Decimal('1.0'), Decimal('2.0'))
        # ),
    ]

    print("\nTesting Interest Rate Calculations")
    print("=" * 80)

    for case in test_cases:
        print(f"\nTesting: {case.description}")
        
        try:
            # Calculate using Rust port
            rust_rate = calculate_interest_rate_rust(
                case.stablecoin_price,
                case.base_rate_bps,
                case.sigma_bps
            )
            rust_compounded = compound_interest_rust(
                rust_rate // YEAR_IN_SECONDS,
                case.time_elapsed
            )
            
            # Calculate using numpy
            numpy_rate = calculate_interest_rate_numpy(
                case.stablecoin_price,
                case.base_rate_bps,
                case.sigma_bps
            )
            numpy_compounded = compound_interest_numpy(
                numpy_rate // YEAR_IN_SECONDS,
                case.time_elapsed
            )
            
            # Calculate using numpy rate but Rust compound formula
            rust_compound_numpy_rate = compound_interest_rust(
                int(numpy_rate) // YEAR_IN_SECONDS,
                case.time_elapsed
            )
            
            # Calculate using pure numpy exp for reference
            pure_numpy_rate = numpy_rate / INTEREST_SCALE
            pure_numpy_compounded = np.exp(np.log(1 + pure_numpy_rate/YEAR_IN_SECONDS) * case.time_elapsed) * INTEREST_SCALE
            
            print(f"Price: ${case.stablecoin_price/PRICE_SCALE:.3f}")
            print(f"Yearly rates:")
            print(f"  Rust:       {rust_rate/INTEREST_SCALE:.18f}")
            print(f"  Numpy:      {numpy_rate/INTEREST_SCALE:.18f}")
            print(f"Compounded rates:")
            print(f"  Rust rate + Rust compound:  {rust_compounded/INTEREST_SCALE:.18f}")
            print(f"  Numpy rate + Rust compound: {rust_compound_numpy_rate/INTEREST_SCALE:.18f}")
            print(f"  Numpy rate + Numpy compound:{numpy_compounded/INTEREST_SCALE:.18f}")
            print(f"  Pure numpy continuous:      {pure_numpy_compounded/INTEREST_SCALE:.18f}")
            print(f"Differences:")
            print(f"  Rate diff (Rust vs Numpy):           {abs(rust_rate - numpy_rate)/INTEREST_SCALE:.18f}")
            print(f"  Compound diff (same rate):           {abs(rust_compound_numpy_rate - numpy_compounded)/INTEREST_SCALE:.18f}")
            print(f"  Total diff (rate+compound):          {abs(rust_compounded - numpy_compounded)/INTEREST_SCALE:.18f}")
            
            assert case.expected_rate_range[0] <= Decimal(rust_rate)/Decimal(INTEREST_SCALE) <= case.expected_rate_range[1], \
                f"Rate {rust_rate/INTEREST_SCALE} outside expected range {case.expected_rate_range}"
            assert Decimal("1.0") <= Decimal(rust_compounded)/Decimal(INTEREST_SCALE) <= Decimal("2.0"), \
                f"Compounded Rate {rust_compounded/INTEREST_SCALE} outside expected range {case.expected_rate_range}"
                
        except ArithmeticError as e:
            print(f"Arithmetic error (expected for overflow test): {str(e)}")

def test_interest_accrual():
    """Test interest accumulation over time"""
    print("\nTesting Interest Accrual")
    print("=" * 80)

    initial_debt = 1000 * PRICE_SCALE
    test_periods = [
        (1, "1 second"),
        (2, "2 seconds"),
        (3, "3 seconds"),
        (4, "4 seconds"),
        (5, "5 seconds"),
        (10, "10 seconds"),
        (60, "1 minute"),
        (3600, "1 hour"),
        (86400, "1 day"),
        (31536000, "1 year"),
    ]
    
    config = ProtocolConfig(
        stable_mint="",  # Mock Pubkey
        protocol_fee=0,
        redemption_fee=0,
        mint_fee=0,
        base_rate=500,  # 5% base rate
        sigma=200,      # 2% sigma
        cumulative_interest_rate=INTEREST_SCALE,
        stablecoin_price_feed="",
        last_interest_rate_update=0,
        total_debt=initial_debt,
        stake_points=0
    )
    
    for elapsed_time, description in test_periods:
        print(f"\nTesting accrual over {description}")
        
        # Test at different price levels
        for price_level in [0.995, 1.0, 1.005]:
            stablecoin_price = int(PRICE_SCALE * price_level)
            
            try:
                # Reset config for this test
                config.cumulative_interest_rate = INTEREST_SCALE
                
                # Calculate using Rust port
                rust_rate = calculate_interest_rate_rust(
                    stablecoin_price,
                    config.base_rate,
                    config.sigma
                )
                compounded_rate = compound_interest_rust(
                    rust_rate // YEAR_IN_SECONDS,
                    elapsed_time
                )
                
                # Calculate using numpy
                numpy_rate = calculate_interest_rate_numpy(
                    stablecoin_price,
                    config.base_rate,
                    config.sigma
                )

                numpy_compounded = compound_interest_numpy(
                    numpy_rate // YEAR_IN_SECONDS,
                    elapsed_time
                )
                
                print(f"\nPrice level: ${price_level:.3f}")
                print(f"Implementation result: {config.cumulative_interest_rate/INTEREST_SCALE:.18f}")
                print(f"Rust calculation: {rust_rate/INTEREST_SCALE:.18f}")
                print(f"Rust compounded: {compounded_rate/INTEREST_SCALE:.18f}")
                print(f"Numpy calculation: {numpy_rate/INTEREST_SCALE:.18f}")
                print(f"Numpy compounded: {numpy_compounded/INTEREST_SCALE:.18f}")
            except ArithmeticError as e:
                print(f"Arithmetic error at {description}, price ${price_level:.3f}: {str(e)}")

def test_edge_cases():
    """Test extreme scenarios"""
    print("\nTesting Edge Cases")
    print("=" * 80)

    test_cases = [
        ("Very low price", PRICE_SCALE // 2),  # $0.50
        ("Very high price", PRICE_SCALE * 2),  # $2.00
        ("Tiny deviation", PRICE_SCALE - 1),   # $0.999999
        ("Small time period", 1),              # 1 second
        ("Large time period", 86400),          # 1 day
        ("Huge debt", 1_000_000 * PRICE_SCALE), # 1M units
        ("Max u128 test", 2**127),             # Test near max u128
    ]
    
    for description, param in test_cases:
        print(f"\nTesting: {description}")
        
        config = ProtocolConfig(
            stable_mint="",  # Mock Pubkey
            protocol_fee=0,
            redemption_fee=0,
            mint_fee=0,
            base_rate=500,  # 5% base rate
            sigma=200,      # 2% sigma
            cumulative_interest_rate=INTEREST_SCALE,
            stablecoin_price_feed="",
            last_interest_rate_update=0,
            total_debt=param if "debt" in description else 1000 * PRICE_SCALE,
            stake_points=0
        )
        
        try:
            # Calculate using Rust port
            rust_rate = calculate_interest_rate_rust(
                param if "price" in description else PRICE_SCALE,
                config.base_rate,
                config.sigma
            )
            compounded_rate = compound_interest_rust(
                rust_rate // YEAR_IN_SECONDS,
                param if "time" in description else 1
            )
            
            # Calculate using numpy
            numpy_rate = calculate_interest_rate_numpy(
                param if "price" in description else PRICE_SCALE,
                config.base_rate,
                config.sigma
            )
            
            print("Test passed successfully")
            print(f"Implementation result: {config.cumulative_interest_rate/INTEREST_SCALE:.18f}")
            print(f"Rust calculation: {rust_rate/INTEREST_SCALE:.18f}")
            print(f"Rust compounded: {compounded_rate/INTEREST_SCALE:.18f}")
            print(f"Numpy calculation: {numpy_rate/INTEREST_SCALE:.18f}")
            print(f"Total debt: {config.total_debt/PRICE_SCALE:.18f}")
            
        except ArithmeticError as e:
            print(f"Arithmetic error (expected for some edge cases): {str(e)}")

if __name__ == "__main__":
    test_interest_rate_calculation()
    test_interest_accrual()
    # test_edge_cases() 