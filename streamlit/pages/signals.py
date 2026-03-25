import streamlit as st
import pandas as pd
import sys
sys.path.append('..')
from utils.db import get_signals, get_active_account_ids
import plotly.express as px

st.set_page_config(page_title="Signals", layout="wide")
st.title("Signals")

account_ids = get_active_account_ids()

col1, col2, col3 = st.columns(3)
with col1:
    view = st.selectbox("Account", ["All"] + account_ids)
with col2:
    signal_type_filter = st.selectbox("Signal Type", ["All", "buy", "sell"])
with col3:
    limit = st.number_input("Limit", min_value=10, max_value=1000, value=100)

account_filter = None if view == "All" else view
signals_df = get_signals(account_id=account_filter, limit=limit)

if signals_df.empty:
    st.info("No signals recorded yet")
    st.stop()

if signal_type_filter != "All":
    signals_df = signals_df[signals_df['signal_type'] == signal_type_filter]

col1, col2, col3 = st.columns(3)
col1.metric("Total Signals", len(signals_df))
col2.metric("Buy Signals", len(signals_df[signals_df['signal_type'] == 'buy']))
col3.metric("Sell Signals", len(signals_df[signals_df['signal_type'] == 'sell']))

st.markdown("---")

st.subheader("Signal Distribution by Symbol")
signal_counts = signals_df.groupby(['symbol', 'signal_type']).size().reset_index(name='count')
fig = px.bar(signal_counts, x='symbol', y='count', color='signal_type', barmode='group')
st.plotly_chart(fig, use_container_width=True)

st.markdown("---")

st.subheader("Signal History")
display_cols = ['timestamp', 'symbol', 'signal_type', 'strength', 'strategy']
if view == "All":
    display_cols.insert(1, 'account_id')
st.dataframe(signals_df[display_cols], hide_index=True, use_container_width=True)
