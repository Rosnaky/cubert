from .base import StrategyDefinition


class MeanReversionDefinition(StrategyDefinition):
    def name(self) -> str:
        return "Mean Reversion"

    def strategy_type(self) -> str:
        return "MeanReversion"

    def default_params(self) -> dict:
        return {
            "window": 20,
            "std_devs": 2.0,
            "startup_lookback": "1Hour",
            "startup_bar_limit": 50,
        }

    def param_descriptions(self) -> dict:
        return {
            "window": "Rolling window size for computing the moving average.",
            "std_devs": "Number of standard deviations from the mean to trigger a signal.",
            "startup_lookback": "Bar timeframe used during historical warmup before live trading begins.",
            "startup_bar_limit": "Number of historical bars to fetch during warmup.",
        }

    def render_params(self, st) -> dict:
        descs = self.param_descriptions()

        col1, col2 = st.columns(2)
        with col1:
            window = st.number_input("Window Size", min_value=5, value=20)
            st.caption(descs["window"])

            startup_lookback = st.selectbox(
                "Startup Timeframe", ["1Min", "5Min", "15Min", "30Min", "1Hour", "1Day"], index=4
            )
            st.caption(descs["startup_lookback"])

        with col2:
            std_devs = st.number_input("Std Deviations", min_value=0.5, value=2.0, step=0.1, format="%.1f")
            st.caption(descs["std_devs"])

            startup_bar_limit = st.number_input("Startup Bar Limit", min_value=10, value=50)
            st.caption(descs["startup_bar_limit"])

        return {
            "window": window,
            "std_devs": std_devs,
            "startup_lookback": startup_lookback,
            "startup_bar_limit": startup_bar_limit,
        }
