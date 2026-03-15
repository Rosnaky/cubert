import pandas as pd
import streamlit as st
import plotly.express as px
import plotly.graph_objects as go
from utils.db import (
    get_account_stats,
    get_signals,
    get_trades,
    get_account_history,
    get_strategy_performance,
    get_signal_stats,
    get_active_account_ids,
)

st.set_page_config(page_title="Cubert Dashboard", layout="wide")
st.title("Cubert Trading Dashboard")

if st.button("Refresh"):
    st.rerun()

account_ids = get_active_account_ids()

if not account_ids:
    st.info("No active accounts. Go to Accounts to create one.")
    st.stop()

account_id = st.selectbox("Account", account_ids)

stats = get_account_stats(account_id)

if not stats:
    st.info(f"No data for {account_id} yet. The engine needs to run at least one tick.")
    st.stop()

# ============ Account Overview ============
st.header("Account Overview")

col1, col2, col3, col4 = st.columns(4)
col1.metric("Equity", f"${stats['equity']:,.2f}")
col2.metric("Cash", f"${stats['cash']:,.2f}")
col3.metric("Trades", stats['total_trades'])
col4.metric("Signals", stats['total_signals'])

# ============ Equity Curve ============
st.header("Equity Curve")

history = get_account_history(account_id, 1000)
if not history.empty:
    history['timestamp'] = pd.to_datetime(history['timestamp'], format='ISO8601')
    fig = px.line(history, x='timestamp', y='equity')
    fig.update_layout(xaxis_title="Time", yaxis_title="Equity ($)")
    st.plotly_chart(fig, use_container_width=True)
else:
    st.info("No account history yet")

# ============ Strategy Performance ============
st.header("Strategy Performance")

col1, col2 = st.columns(2)

with col1:
    st.subheader("Trade Count by Strategy")
    perf = get_strategy_performance(account_id)
    if not perf.empty:
        fig = px.bar(perf, x='strategy', y='trade_count')
        st.plotly_chart(fig, use_container_width=True)
    else:
        st.info("No trades yet")

with col2:
    st.subheader("Signal Stats")
    signal_stats = get_signal_stats(account_id)
    if not signal_stats.empty:
        st.dataframe(signal_stats, use_container_width=True)
    else:
        st.info("No signals yet")

# ============ Recent Trades ============
st.header("Recent Trades")

trades = get_trades(account_id=account_id, limit=50)
if not trades.empty:
    trades['value'] = trades['quantity'] * trades['price']
    st.dataframe(
        trades[['timestamp', 'symbol', 'side', 'quantity', 'price', 'value', 'strategy']],
        use_container_width=True, hide_index=True,
    )
else:
    st.info("No trades yet")

# ============ Recent Signals ============
st.header("Recent Signals")

signals = get_signals(account_id=account_id, limit=50)
if not signals.empty:
    st.dataframe(
        signals[['timestamp', 'symbol', 'signal_type', 'strength', 'strategy']],
        use_container_width=True, hide_index=True,
    )

    col1, col2 = st.columns(2)
    with col1:
        fig = px.pie(signals, names='signal_type', title='Signal Distribution')
        st.plotly_chart(fig, use_container_width=True)
    with col2:
        fig = px.histogram(signals, x='strength', nbins=20, title='Signal Strength')
        st.plotly_chart(fig, use_container_width=True)
else:
    st.info("No signals yet")
