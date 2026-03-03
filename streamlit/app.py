import pandas as pd
import streamlit as st
import plotly.express as px
import plotly.graph_objects as go
from utils.db import (
    get_account_stats,
    get_positions,
    get_signals,
    get_broker_trades,
    get_engine_trades,
    get_account_history,
    get_strategy_performance,
    get_signal_stats,
)

st.set_page_config(page_title="Cubert Dashboard", layout="wide")
st.title("🤖 Cubert Trading Dashboard")

# Auto-refresh
if st.button("🔄 Refresh"):
    st.rerun()

# ============ Account Overview ============
st.header("📊 Account Overview")

stats = get_account_stats()
col1, col2, col3, col4 = st.columns(4)

with col1:
    st.metric("Equity", f"${stats['equity']:,.2f}")
with col2:
    st.metric("Cash", f"${stats['cash']:,.2f}")
with col3:
    st.metric("Unrealized P&L", f"${stats['unrealized_pnl']:,.2f}")
with col4:
    st.metric("Positions", stats['positions_count'])

col5, col6 = st.columns(2)
with col5:
    st.metric("Total Trades", stats['total_trades'])
with col6:
    st.metric("Total Signals", stats['total_signals'])

# ============ Equity Curve ============
st.header("📈 Equity Curve")

history = get_account_history(1000)
if not history.empty:
    history['timestamp'] = pd.to_datetime(history['timestamp'])
    fig = px.line(history, x='timestamp', y='equity', title='Account Equity Over Time')
    fig.update_layout(xaxis_title="Time", yaxis_title="Equity ($)")
    st.plotly_chart(fig, use_container_width=True)
else:
    st.info("No account history yet")

# ============ Positions ============
st.header("💼 Current Positions")

positions = get_positions()
if not positions.empty:
    positions['market_value'] = positions['quantity'] * positions['current_price']
    positions['pnl'] = (positions['current_price'] - positions['avg_entry_price']) * positions['quantity']
    positions['pnl_pct'] = ((positions['current_price'] / positions['avg_entry_price']) - 1) * 100
    
    st.dataframe(
        positions[['symbol', 'quantity', 'avg_entry_price', 'current_price', 'market_value', 'pnl', 'pnl_pct']],
        use_container_width=True,
        column_config={
            'avg_entry_price': st.column_config.NumberColumn('Avg Entry', format='$%.2f'),
            'current_price': st.column_config.NumberColumn('Current', format='$%.2f'),
            'market_value': st.column_config.NumberColumn('Value', format='$%.2f'),
            'pnl': st.column_config.NumberColumn('P&L', format='$%.2f'),
            'pnl_pct': st.column_config.NumberColumn('P&L %', format='%.2f%%'),
        }
    )
else:
    st.info("No open positions")

# ============ Strategy Performance ============
st.header("🎯 Strategy Performance")

col1, col2 = st.columns(2)

with col1:
    st.subheader("Trade Count by Strategy")
    perf = get_strategy_performance()
    if not perf.empty:
        fig = px.bar(perf, x='strategy', y='trade_count', title='Trades per Strategy')
        st.plotly_chart(fig, use_container_width=True)
    else:
        st.info("No trades yet")

with col2:
    st.subheader("Signal Stats")
    signal_stats = get_signal_stats()
    if not signal_stats.empty:
        st.dataframe(signal_stats, use_container_width=True)
    else:
        st.info("No signals yet")

# ============ Recent Trades ============
st.header("📜 Recent Trades")

tab1, tab2 = st.tabs(["Broker Trades (Executed)", "Engine Trades (With Strategy)"])

with tab1:
    broker_trades = get_broker_trades(50)
    if not broker_trades.empty:
        broker_trades['value'] = broker_trades['quantity'] * broker_trades['price']
        st.dataframe(broker_trades, use_container_width=True)
    else:
        st.info("No trades yet")

with tab2:
    engine_trades = get_engine_trades(50)
    if not engine_trades.empty:
        engine_trades['value'] = engine_trades['quantity'] * engine_trades['price']
        st.dataframe(engine_trades, use_container_width=True)
    else:
        st.info("No trades yet")

# ============ Recent Signals ============
st.header("📡 Recent Signals")

signals = get_signals(50)
if not signals.empty:
    st.dataframe(signals, use_container_width=True)
    
    # Signal distribution
    col1, col2 = st.columns(2)
    with col1:
        fig = px.pie(signals, names='signal_type', title='Signal Distribution')
        st.plotly_chart(fig, use_container_width=True)
    with col2:
        fig = px.histogram(signals, x='strength', nbins=20, title='Signal Strength Distribution')
        st.plotly_chart(fig, use_container_width=True)
else:
    st.info("No signals yet")
