import numpy as np
import matplotlib.pyplot as plt
from dataclasses import dataclass, field
from typing import List, Tuple, Optional, Callable, Union
import pandas as pd
from pathlib import Path
from datetime import datetime

# Constants for rate limits, clamps between these below values
MIN_RATE = 0.01  # 1% APR
MAX_RATE = 0.30  # 30% APR

@dataclass
class ExponentialRateParams:
    base_rate: float = 0.05  #base rate (rate0 in crvusd)
    sigma: float = 0.02      

@dataclass
class GeometricRateParams:
    base_rate: float = 0.05  #base rate (rate0 in crvusd)
    sigma: float = 0.02
    power: float = 2.0

@dataclass
class EMAExponentialRateParams(ExponentialRateParams):
    alpha: float = 0.01  # smoothing factor (0 to 1)

@dataclass
class SimulationParams:
    initial_price: float = 1.0
    price_volatility: float = 0.0001
    simulation_days: int = 365
    steps_per_day: int = 24  # hourly steps
    random_seed: Optional[int] = None
    experiment_name: str = "default"
    rate_params: Union[ExponentialRateParams, GeometricRateParams, EMAExponentialRateParams] = field(default_factory=GeometricRateParams)

@dataclass
class RateModelConfig:
    """Configuration for a rate model including its function and parameters"""
    name: str
    model_func: Callable[[float, Union[ExponentialRateParams, GeometricRateParams, EMAExponentialRateParams]], float]
    params: Union[ExponentialRateParams, GeometricRateParams, EMAExponentialRateParams]
    
    def __str__(self):
        if isinstance(self.params, EMAExponentialRateParams):
            params_str = f"rate0={self.params.base_rate:.3f}, σ={self.params.sigma:.3f}, α={self.params.alpha:.3f}"
        elif isinstance(self.params, ExponentialRateParams):
            params_str = f"rate0={self.params.base_rate:.3f}, σ={self.params.sigma:.3f}"
        elif isinstance(self.params, GeometricRateParams):
            params_str = f"rate0={self.params.base_rate:.3f}, σ={self.params.sigma:.3f}, power={self.params.power:.1f}"
        return f"{self.name} ({params_str})"

class InterestRateModels:
    @staticmethod
    def exponential_rate(price: float, rate_params: ExponentialRateParams) -> float:
        """
        Calculate rate using exponential formula:
        rate = rate0 * exp((1 - p) / sigma)
        """
        power = (1.0 - price) / rate_params.sigma
        rate = rate_params.base_rate * np.exp(power)
        return np.clip(rate, MIN_RATE, MAX_RATE)

    @staticmethod
    def geometric_rate(price: float, rate_params: GeometricRateParams) -> float:
        """
        I didnt like this too much to be honest. just skip it.
        ----------------------------------------------------------------

        Calculate rate using geometric formula:
        deviation = (1-p)/sigma
        x = (1 + deviation)^power - 1
        rate = rate0 * (1 + x) for p <= 1
        rate = rate0 * (1 - x) for p > 1
        
        This ensures:
        1. Continuity at p = 1
        2. Symmetry around the peg
        3. Smooth transitions
        """
        deviation = (1.0 - price) / rate_params.sigma
        x = (1 + abs(deviation)) ** rate_params.power - 1
        
        if price <= 1.0:
            rate = rate_params.base_rate * (1 + x)
        else:
            rate = rate_params.base_rate * (1 - x)
            
        return np.clip(rate, MIN_RATE, MAX_RATE)

    @staticmethod
    def ema_exponential_rate(price: float, rate_params: EMAExponentialRateParams, prev_rate: Optional[float] = None) -> float:
        """
        Calculate rate using exponential formula with EMA smoothing:
        1. Calculate raw_rate using exponential formula
        2. Apply EMA smoothing: rate_t = α * raw_rate + (1-α) * rate_(t-1)
        
        Args:
            price: Current price
            rate_params: Parameters including base_rate, sigma, and alpha
            prev_rate: Previous rate value for EMA calculation
        """
        # Calculate raw rate using exponential formula
        raw_rate = InterestRateModels.exponential_rate(price, rate_params)
        
        # Apply ema smoothening if we have a previous rate
        if prev_rate is not None:
            rate = rate_params.alpha * raw_rate + (1 - rate_params.alpha) * prev_rate
        else:
            rate = raw_rate
            
        return np.clip(rate, MIN_RATE, MAX_RATE)

class StablecoinSimulation:
    def __init__(self, 
                 params: SimulationParams,
                 rate_model: Callable[[float, Union[ExponentialRateParams, GeometricRateParams, EMAExponentialRateParams]], float] = InterestRateModels.geometric_rate):
        self.params = params
        self.rate_model = rate_model
        self.prices: List[float] = []
        self.rates: List[float] = []
        self.times: List[float] = []
        self.prev_rate: Optional[float] = None
        
        if params.random_seed is not None:
            np.random.seed(params.random_seed)
        
    def calculate_rate(self, current_price: float) -> float:
        """Calculate interest rate using the selected rate model"""
        if self.rate_model == InterestRateModels.ema_exponential_rate:
            rate = self.rate_model(current_price, self.params.rate_params, self.prev_rate)
            self.prev_rate = rate
            return rate
        return self.rate_model(current_price, self.params.rate_params)
    
    def simulate(self) -> Tuple[List[float], List[float], List[float]]:
        current_price = self.params.initial_price
        current_rate = self.params.rate_params.base_rate
        total_steps = self.params.simulation_days * self.params.steps_per_day
        
        for step in range(total_steps):
            # Simulate price movement with Brownian motion
            price_change = np.random.normal(0, self.params.price_volatility)
            current_price *= (1 + price_change)
            
            current_rate = self.calculate_rate(current_price)
            
            self.times.append(step / self.params.steps_per_day)
            self.prices.append(current_price)
            self.rates.append(current_rate)
        
        return self.times, self.prices, self.rates
    
    def plot_results(self):
        output_dir = Path('research/results') / self.params.experiment_name
        output_dir.mkdir(parents=True, exist_ok=True)
        
        fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 8))
        
        # Plot price
        ax1.plot(self.times, self.prices, label='Stablecoin Price')
        ax1.axhline(y=1.0, color='r', linestyle='--', alpha=0.3)
        ax1.set_ylabel('Price (USD)')
        ax1.set_title('Stablecoin Price Over Time')
        ax1.legend()
        ax1.grid(True)
        
        # Plot interest rate
        ax2.plot(self.times, self.rates, label='Interest Rate', color='orange')
        ax2.set_ylabel('Interest Rate')
        ax2.set_xlabel('Time (days)')
        ax2.set_title('Interest Rate Over Time')
        ax2.legend()
        ax2.grid(True)
        
        plt.tight_layout()
        
        # Save plot with descriptive name
        plot_name = f"sigma_{self.params.rate_params.sigma}_base_rate_{self.params.rate_params.base_rate}"
        if self.params.random_seed is not None:
            plot_name += f"_seed_{self.params.random_seed}"
        
        plt.savefig(output_dir / f"{plot_name}.png")
        plt.close()

def compare_rate_models(rate_models: List[RateModelConfig], base_params: SimulationParams):
    """Run simulations with different rate models and plot them together"""
    output_dir = Path('research/results/rate_model_comparison')
    output_dir.mkdir(parents=True, exist_ok=True)
    
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 10))
    
    # Run first simulation to get price history
    first_sim = StablecoinSimulation(base_params)
    first_sim.simulate()
    
    # Plot price
    ax1.plot(first_sim.times, first_sim.prices, label='Stablecoin Price')
    
    # Plot rates for all models
    for model_config in rate_models:
        # Create new params with current rate model parameters
        params = SimulationParams(
            initial_price=base_params.initial_price,
            price_volatility=base_params.price_volatility,
            simulation_days=base_params.simulation_days,
            steps_per_day=base_params.steps_per_day,
            random_seed=base_params.random_seed,
            experiment_name=base_params.experiment_name,
            rate_params=model_config.params
        )
        
        # Run simulation with current rate model
        sim = StablecoinSimulation(params, rate_model=model_config.model_func)
        sim.simulate()
        
        # Plot only rates
        ax2.plot(sim.times, np.array(sim.rates) * 100, label=str(model_config))
    
    # Configure plots
    ax1.axhline(y=1.0, color='r', linestyle='--', alpha=0.3)
    ax1.set_ylabel('Price (USD)')
    ax1.set_title('Stablecoin Price Over Time')
    ax1.legend(loc='upper left')
    ax1.grid(True, alpha=0.3)  # Lighter grid
    
    ax2.set_ylabel('Interest Rate (%)')
    ax2.set_xlabel('Time (days)')
    ax2.set_title('Interest Rate Over Time')
    ax2.legend(bbox_to_anchor=(1.05, 1), loc='upper left')
    ax2.grid(True, alpha=0.3)  # Lighter grid
    
    # Add seed information to the plot
    seed_text = f"Random Seed: {base_params.random_seed}" if base_params.random_seed is not None else "No Seed"
    fig.text(0.02, 0.02, seed_text, fontsize=8, alpha=0.7)
    
    plt.tight_layout()
    
    # Create unique filename with just timestamp
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = f"rate_comparison_{timestamp}.png"
    
    plt.savefig(output_dir / filename, bbox_inches='tight', dpi=300)
    plt.close()

def main():
    # Define different rate models with their parameters
    rate_models = [
        RateModelConfig(
            name="Exponential 1",  # sublinear geometric growth
            model_func=InterestRateModels.exponential_rate,
            params=ExponentialRateParams(base_rate=0.05, sigma=0.005))
        ,
        RateModelConfig(
            name="Exponential 2",  # sublinear geometric growth
            model_func=InterestRateModels.exponential_rate,
            params=ExponentialRateParams(base_rate=0.05, sigma=0.01))
        ,
        RateModelConfig(
            name="Exponential 3",  # sublinear geometric growth
            model_func=InterestRateModels.exponential_rate,
            params=ExponentialRateParams(base_rate=0.05, sigma=0.02))
        ,
        # RateModelConfig(
        #     name="Geometric 0.5",  # sublinear geometric growth
        #     model_func=InterestRateModels.geometric_rate,
        #     params=GeometricRateParams(base_rate=0.05, sigma=0.02, power=0.5)
        # ),
        # RateModelConfig(
        #     name="Geometric 1.5",  # superlinear geometric growth
        #     model_func=InterestRateModels.geometric_rate,
        #     params=GeometricRateParams(base_rate=0.05, sigma=0.02, power=1.5)
        # ),
        RateModelConfig(
            name="EMA Exponential 2",
            model_func=InterestRateModels.ema_exponential_rate,
            params=EMAExponentialRateParams(base_rate=0.05, sigma=0.01, alpha=0.03)
        )
    ]
    
    # Run comparison
    base_params = SimulationParams(
        experiment_name="rate_model_comparison",
        random_seed=57,
        simulation_days=100
    )
    
    compare_rate_models(rate_models, base_params)
    
    # # signle run for testing
    # params = SimulationParams(
    #     experiment_name="single_run",
    #     random_seed=42
    # )
    # sim = StablecoinSimulation(params)
    # sim.simulate()
    # sim.plot_results()

if __name__ == "__main__":
    main() 