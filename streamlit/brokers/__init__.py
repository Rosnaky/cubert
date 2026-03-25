from .paper_alpaca import PaperAlpacaDefinition
from .alpaca_live import AlpacaLiveDefinition

_ALL_BROKERS = [
    PaperAlpacaDefinition(),
    AlpacaLiveDefinition(),
]

BROKER_REGISTRY = {b.broker_type(): b for b in _ALL_BROKERS}

def get_broker_types():
    return list(BROKER_REGISTRY.keys())

def get_broker_definition(broker_type: str):
    return BROKER_REGISTRY.get(broker_type)
