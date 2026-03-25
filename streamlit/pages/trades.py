import streamlit as st
import pandas as pd
import sys
sys.path.append('..')
from utils.db import get_trades, get_active_account_ids
import plotly.express as px

st.set_page_config(page_title="Trades", layout="wide")
st.title("Trades")

account_ids = get_active_account_ids()

col1, col2 = st.columns(2)
with col1:
    view = st.selectbox("Account", ["All"] + account_ids)
with col2:
    limit = st.number_input("Limit", min_value=10, max_value=1000, value=200)

account_filter = None if view == "All" else view
trades_df = get_trades(account_id=account_filter, limit=limit)

if trades_df.empty:
    st.info("No trades yet")
    st.stop()

trades_df['value'] = trades_df['quantity'] * trades_df['price']

col1, col2, col3, col4 = st.columns(4)
col1.metric("Total Trades", len(trades_df))

buy_value = trades_df[trades_df['side'] == 'buy']['value'].sum()
col2.metric("Total Bought", f"${buy_value:,.2f}")

sell_value = trades_df[trades_df['side'] == 'sell']['value'].sum()
col3.metric("Total Sold", f"${sell_value:,.2f}")

pnl = sell_value - buy_value
col4.metric("Net P&L", f"${pnl:,.2f}")

st.markdown("---")

st.subheader("Trade Volume by Symbol")
volume_by_symbol = trades_df.groupby('symbol')['value'].sum().reset_index()
fig = px.pie(volume_by_symbol, values='value', names='symbol')
st.plotly_chart(fig, use_container_width=True)

st.markdown("---")

st.subheader("Trade History")
display_cols = ['timestamp', 'symbol', 'side', 'quantity', 'price', 'strategy']
if view == "All":
    display_cols.insert(1, 'account_id')
st.dataframe(trades_df[display_cols], hide_index=True, use_container_width=True)
