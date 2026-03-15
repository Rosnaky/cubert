from .base import StrategyDefinition


class MomentumDefinition(StrategyDefinition):
    def name(self) -> str:
        return "Momentum"

    def strategy_type(self) -> str:
        return "Momentum"

    def default_params(self) -> dict:
        return {
            "lookback_period": 20,
            "threshold": 0.02,
            "startup_lookback": "1Hour",
            "startup_bar_limit": 50,
        }

    def param_descriptions(self) -> dict:
        return {
            "lookback_period": "Number of bars to compare current price against for momentum calculation.",
            "threshold": "Minimum momentum percentage to trigger a signal. Higher = fewer trades.",
            "startup_lookback": "Bar timeframe used during historical warmup before live trading begins.",
            "startup_bar_limit": "Number of historical bars to fetch during warmup.",
        }

    def render_params(self, st) -> dict:
        descs = self.param_descriptions()

        col1, col2 = st.columns(2)
        with col1:
            lookback = st.number_input("Lookback Period", min_value=1, value=20)
            st.caption(descs["lookback_period"])

            startup_lookback = st.selectbox(
                "Startup Timeframe", ["1Min", "5Min", "15Min", "30Min", "1Hour", "1Day"], index=4
            )
            st.caption(descs["startup_lookback"])

        with col2:
            threshold = st.number_input("Threshold", min_value=0.001, value=0.02, step=0.005, format="%.3f")
            st.caption(descs["threshold"])

            startup_bar_limit = st.number_input("Startup Bar Limit", min_value=10, value=50)
            st.caption(descs["startup_bar_limit"])

        return {
            "lookback_period": lookback,
            "threshold": threshold,
            "startup_lookback": startup_lookback,
            "startup_bar_limit": startup_bar_limit,
        }
