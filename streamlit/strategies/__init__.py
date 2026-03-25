from .momentum import MomentumDefinition
from .mean_reversion import MeanReversionDefinition

_ALL_STRATEGIES = [
    MomentumDefinition(),
    MeanReversionDefinition(),
]

STRATEGY_REGISTRY = {s.strategy_type(): s for s in _ALL_STRATEGIES}

def get_strategy_types():
    return list(STRATEGY_REGISTRY.keys())

def get_strategy_definition(strategy_type: str):
    return STRATEGY_REGISTRY.get(strategy_type)
