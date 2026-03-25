from .base import BrokerDefinition


class PaperAlpacaDefinition(BrokerDefinition):
    def name(self) -> str:
        return "Paper (Alpaca Data)"

    def broker_type(self) -> str:
        return "paper"

    def description(self) -> str:
        return "Simulated trading with real Alpaca market data. No real money."

    def render_config(self, st) -> dict:
        starting_cash = st.number_input(
            "Starting Cash", min_value=1000.0, value=100000.0, step=10000.0, format="%.2f"
        )
        return {"starting_cash": starting_cash}
