from .base import StrategyDefinition


class MeanReversionDefinition(StrategyDefinition):
    def name(self) -> str:
        return "Mean Reversion"

    def strategy_type(self) -> str:
        return "MeanReversion"

    def default_params(self) -> dict:
        return {
            "window": 60,
            "zscore_entry": 2.0,
            "zscore_exit": 0.5,
            "min_half_life": 2.0,
            "max_half_life": 50.0,
            "adf_lags": 1,
            "recompute_interval": 5,
            "max_buffer": 500,
            "startup_lookback": "1Hour",
            "startup_bar_limit": 500,
        }

    def param_descriptions(self) -> dict:
        return {
            "window": "Rolling window size for z-score and OU parameter estimation.",
            "zscore_entry": "Z-score threshold to enter a trade. Higher = fewer but stronger signals.",
            "zscore_exit": "Z-score threshold to exit. When price reverts past equilibrium, close the position.",
            "min_half_life": "Minimum mean-reversion half-life in bars. Below this, signal is noise.",
            "max_half_life": "Maximum mean-reversion half-life in bars. Above this, capital is tied up too long.",
            "adf_lags": "Number of lags for the Augmented Dickey-Fuller stationarity test.",
            "recompute_interval": "Recompute GPU signals every N bars. Lower = more responsive, higher = less GPU usage.",
            "max_buffer": "Maximum price history to keep per symbol. Also controls startup bar fetch.",
            "startup_lookback": "Bar timeframe used during historical warmup before live trading begins.",
            "startup_bar_limit": "Number of historical bars to fetch during warmup.",
        }

    def render_params(self, st) -> dict:
        descs = self.param_descriptions()

        col1, col2 = st.columns(2)
        with col1:
            window = st.number_input("Window Size", min_value=5, value=60)
            st.caption(descs["window"])

            zscore_entry = st.number_input("Z-Score Entry", min_value=0.5, value=2.0, step=0.1, format="%.1f")
            st.caption(descs["zscore_entry"])

            min_half_life = st.number_input("Min Half-Life", min_value=1.0, value=2.0, step=1.0, format="%.1f")
            st.caption(descs["min_half_life"])

            adf_lags = st.number_input("ADF Lags", min_value=1, value=1)
            st.caption(descs["adf_lags"])

            startup_lookback = st.selectbox(
                "Startup Timeframe", ["1Min", "5Min", "15Min", "30Min", "1Hour", "1Day"], index=4
            )
            st.caption(descs["startup_lookback"])

        with col2:
            zscore_exit = st.number_input("Z-Score Exit", min_value=0.0, value=0.5, step=0.1, format="%.1f")
            st.caption(descs["zscore_exit"])

            max_half_life = st.number_input("Max Half-Life", min_value=5.0, value=50.0, step=5.0, format="%.1f")
            st.caption(descs["max_half_life"])

            recompute_interval = st.number_input("Recompute Interval", min_value=1, value=5)
            st.caption(descs["recompute_interval"])

            max_buffer = st.number_input("Max Buffer", min_value=50, value=500, step=50)
            st.caption(descs["max_buffer"])

            startup_bar_limit = st.number_input("Startup Bar Limit", min_value=10, value=500)
            st.caption(descs["startup_bar_limit"])

        return {
            "window": window,
            "zscore_entry": zscore_entry,
            "zscore_exit": zscore_exit,
            "min_half_life": min_half_life,
            "max_half_life": max_half_life,
            "adf_lags": adf_lags,
            "recompute_interval": recompute_interval,
            "max_buffer": max_buffer,
            "startup_lookback": startup_lookback,
            "startup_bar_limit": startup_bar_limit,
        }
        