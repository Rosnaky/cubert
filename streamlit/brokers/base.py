from abc import ABC, abstractmethod


class BrokerDefinition(ABC):
    @abstractmethod
    def name(self) -> str:
        pass

    @abstractmethod
    def broker_type(self) -> str:
        pass

    @abstractmethod
    def render_config(self, st) -> dict:
        """Render Streamlit inputs and return config dict. Return None to skip."""
        pass

    @abstractmethod
    def description(self) -> str:
        pass
