import streamlit as st
import sys
sys.path.append('..')
from utils.db import get_trades
import plotly.express as px
import pandas as pd

st.title("💼 Trades")

# Get data
trades_df = get_trades(limit=200)

if not trades_df.empty:
    # Calculate P&L
    trades_df['value'] = trades_df['quantity'] * trades_df['price']
    
    # Stats
    col1, col2, col3, col4 = st.columns(4)
    
    with col1:
        st.metric("Total Trades", len(trades_df))
    
    with col2:
        buy_value = trades_df[trades_df['side'] == 'buy']['value'].sum()
        st.metric("Total Bought", f"${buy_value:,.2f}")
    
    with col3:
        sell_value = trades_df[trades_df['side'] == 'sell']['value'].sum()
        st.metric("Total Sold", f"${sell_value:,.2f}")
    
    with col4:
        pnl = sell_value - buy_value
        st.metric("Net P&L", f"${pnl:,.2f}", delta=f"{pnl:,.2f}")
    
    st.markdown("---")
    
    # Trade volume by symbol
    st.subheader("Trade Volume by Symbol")
    volume_by_symbol = trades_df.groupby('symbol')['value'].sum().reset_index()
    fig = px.pie(volume_by_symbol, values='value', names='symbol', title='Trade Volume Distribution')
    st.plotly_chart(fig, use_container_width=True)
    
    st.markdown("---")
    
    # Trade history
    st.subheader("Trade History")
    st.dataframe(
        trades_df[['timestamp', 'symbol', 'side', 'quantity', 'price', 'strategy']],
        hide_index=True,
        use_container_width=True
    )
else:
    st.info("No trades yet")
    