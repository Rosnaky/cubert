import streamlit as st
import pandas as pd
import sys
sys.path.append('..')
from utils.db import (
    get_account_stats,
    get_engine_trades,
    get_signals,
    get_positions,
    get_account_history,
)
import plotly.express as px
import plotly.graph_objects as go

st.set_page_config(page_title="Cubert Overview", layout="wide")
st.title("📊 Overview")

# Auto-refresh button
if st.button("🔄 Refresh"):
    st.rerun()

# Get stats
stats = get_account_stats()

# Metrics row 1
col1, col2, col3, col4 = st.columns(4)

with col1:
    st.metric("💰 Equity", f"${stats['equity']:,.2f}")

with col2:
    st.metric("💵 Cash", f"${stats['cash']:,.2f}")

with col3:
    pnl = stats['unrealized_pnl']
    st.metric("📈 Unrealized P&L", f"${pnl:,.2f}", delta=f"{pnl:,.2f}")

with col4:
    st.metric("💼 Positions", stats['positions_count'])

# Metrics row 2
col1, col2, col3, col4 = st.columns(4)

with col1:
    st.metric("📊 Total Trades", stats['total_trades'])

with col2:
    st.metric("📡 Total Signals", stats['total_signals'])

with col3:
    st.metric("💳 Buying Power", f"${stats['buying_power']:,.2f}")

with col4:
    # Calculate win rate if we have trades
    pass  # Placeholder for future metrics

st.markdown("---")

# Equity curve
st.subheader("📈 Equity Curve")
history = get_account_history(500)
if not history.empty:
    history['timestamp'] = pd.to_datetime(history['timestamp'])
    history = history.sort_values('timestamp')
    
    fig = go.Figure()
    fig.add_trace(go.Scatter(
        x=history['timestamp'],
        y=history['equity'],
        mode='lines',
        name='Equity',
        line=dict(color='#00d4aa', width=2),
        fill='tozeroy',
        fillcolor='rgba(0, 212, 170, 0.1)'
    ))
    fig.update_layout(
        xaxis_title="Time",
        yaxis_title="Equity ($)",
        hovermode='x unified',
        height=300,
        margin=dict(l=0, r=0, t=10, b=0),
    )
    st.plotly_chart(fig, use_container_width=True)
else:
    st.info("No equity history yet")

st.markdown("---")

# Current positions
st.subheader("💼 Current Positions")
positions = get_positions()
if not positions.empty:
    positions['market_value'] = positions['quantity'] * positions['current_price']
    positions['pnl'] = (positions['current_price'] - positions['avg_entry_price']) * positions['quantity']
    positions['pnl_pct'] = ((positions['current_price'] / positions['avg_entry_price']) - 1) * 100
    
    st.dataframe(
        positions[['symbol', 'quantity', 'avg_entry_price', 'current_price', 'market_value', 'pnl', 'pnl_pct']],
        hide_index=True,
        use_container_width=True,
        column_config={
            'symbol': 'Symbol',
            'quantity': st.column_config.NumberColumn('Qty', format='%.2f'),
            'avg_entry_price': st.column_config.NumberColumn('Avg Entry', format='$%.2f'),
            'current_price': st.column_config.NumberColumn('Current', format='$%.2f'),
            'market_value': st.column_config.NumberColumn('Value', format='$%.2f'),
            'pnl': st.column_config.NumberColumn('P&L', format='$%.2f'),
            'pnl_pct': st.column_config.NumberColumn('P&L %', format='%.2f%%'),
        }
    )
else:
    st.info("No open positions")

st.markdown("---")

# Recent activity
col1, col2 = st.columns(2)

with col1:
    st.subheader("🔄 Recent Trades")
    trades_df = get_engine_trades(limit=10)
    if not trades_df.empty:
        trades_df['value'] = trades_df['quantity'] * trades_df['price']
        st.dataframe(
            trades_df[['symbol', 'side', 'quantity', 'price', 'value', 'timestamp']],
            hide_index=True,
            use_container_width=True,
            column_config={
                'symbol': 'Symbol',
                'side': 'Side',
                'quantity': st.column_config.NumberColumn('Qty', format='%.2f'),
                'price': st.column_config.NumberColumn('Price', format='$%.2f'),
                'value': st.column_config.NumberColumn('Value', format='$%.2f'),
                'timestamp': 'Time',
            }
        )
    else:
        st.info("No trades yet")

with col2:
    st.subheader("📡 Recent Signals")
    signals_df = get_signals(limit=10)
    if not signals_df.empty:
        st.dataframe(
            signals_df[['symbol', 'signal_type', 'strength', 'strategy', 'timestamp']],
            hide_index=True,
            use_container_width=True,
            column_config={
                'symbol': 'Symbol',
                'signal_type': 'Signal',
                'strength': st.column_config.NumberColumn('Strength', format='%.2f'),
                'strategy': 'Strategy',
                'timestamp': 'Time',
            }
        )
    else:
        st.info("No signals yet")
