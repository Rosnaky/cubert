from .base import BrokerDefinition


class AlpacaLiveDefinition(BrokerDefinition):
    def name(self) -> str:
        return "Alpaca Live"

    def broker_type(self) -> str:
        return "live"

    def description(self) -> str:
        return "Live trading via Alpaca API. Uses real money."

    def render_config(self, st) -> dict:
        st.warning("This will use real money. Ensure API keys are configured in .env")
        return {}
