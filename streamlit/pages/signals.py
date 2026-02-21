import streamlit as st
import sys
sys.path.append('..')
from utils.db import get_signals
import plotly.express as px
import pandas as pd

st.title("📡 Signals")

# Filters
col1, col2, col3 = st.columns(3)

with col1:
    symbol_filter = st.selectbox("Symbol", ["All", "AAPL", "MSFT", "TSLA"])

with col2:
    signal_type_filter = st.selectbox("Signal Type", ["All", "buy", "sell"])

with col3:
    limit = st.number_input("Records to show", min_value=10, max_value=1000, value=100)

# Get data
signals_df = get_signals(limit=limit)

if not signals_df.empty:
    # Apply filters
    if symbol_filter != "All":
        signals_df = signals_df[signals_df['symbol'] == symbol_filter]
    
    if signal_type_filter != "All":
        signals_df = signals_df[signals_df['signal_type'] == signal_type_filter]
    
    # Stats
    col1, col2, col3 = st.columns(3)
    
    with col1:
        st.metric("Total Signals", len(signals_df))
    
    with col2:
        buy_count = len(signals_df[signals_df['signal_type'] == 'buy'])
        st.metric("Buy Signals", buy_count)
    
    with col3:
        sell_count = len(signals_df[signals_df['signal_type'] == 'sell'])
        st.metric("Sell Signals", sell_count)
    
    st.markdown("---")
    
    # Signal distribution
    st.subheader("Signal Distribution by Symbol")
    signal_counts = signals_df.groupby(['symbol', 'signal_type']).size().reset_index(name='count')
    fig = px.bar(signal_counts, x='symbol', y='count', color='signal_type', barmode='group')
    st.plotly_chart(fig, use_container_width=True)
    
    st.markdown("---")
    
    # Data table
    st.subheader("Signal History")
    st.dataframe(
        signals_df[['timestamp', 'symbol', 'signal_type', 'strength', 'strategy']],
        hide_index=True,
        use_container_width=True
    )
else:
    st.info("No signals recorded yet")
    