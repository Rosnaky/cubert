import streamlit as st
import sys
sys.path.append('..')
from utils.db import get_account_stats, get_trades, get_signals
import plotly.graph_objects as go
from datetime import datetime, timedelta

st.title("📊 Overview")

# Get stats
stats = get_account_stats()

# Metrics
col1, col2, col3, col4 = st.columns(4)

with col1:
    st.metric("💰 Equity", f"${stats['equity']:,.2f}")

with col2:
    st.metric("💵 Cash", f"${stats['cash']:,.2f}")

with col3:
    st.metric("📊 Total Trades", stats['total_trades'])

with col4:
    st.metric("📡 Total Signals", stats['total_signals'])

st.markdown("---")

# Recent activity
col1, col2 = st.columns(2)

with col1:
    st.subheader("Recent Trades")
    trades_df = get_trades(limit=5)
    if not trades_df.empty:
        st.dataframe(
            trades_df[['symbol', 'side', 'quantity', 'price', 'timestamp']],
            hide_index=True,
            use_container_width=True
        )
    else:
        st.info("No trades yet")

with col2:
    st.subheader("Recent Signals")
    signals_df = get_signals(limit=5)
    if not signals_df.empty:
        st.dataframe(
            signals_df[['symbol', 'signal_type', 'strength', 'strategy', 'timestamp']],
            hide_index=True,
            use_container_width=True
        )
    else:
        st.info("No signals yet")
        