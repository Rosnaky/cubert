from abc import ABC, abstractmethod


class StrategyDefinition(ABC):
    @abstractmethod
    def name(self) -> str:
        pass

    @abstractmethod
    def strategy_type(self) -> str:
        pass

    @abstractmethod
    def default_params(self) -> dict:
        pass

    @abstractmethod
    def param_descriptions(self) -> dict:
        pass

    @abstractmethod
    def render_params(self, st) -> dict:
        pass

    def to_params_json(self, params: dict) -> dict:
        return {"type": self.strategy_type(), **params}
