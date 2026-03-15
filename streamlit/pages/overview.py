import streamlit as st
import pandas as pd
import sys
sys.path.append('..')
from utils.db import get_account_stats, get_trades, get_signals, get_account_history, get_active_account_ids
import plotly.graph_objects as go

st.set_page_config(page_title="Cubert Overview", layout="wide")
st.title("Overview")

if st.button("Refresh"):
    st.rerun()

account_ids = get_active_account_ids()

if not account_ids:
    st.info("No active accounts. Create an account and assign strategies to get started.")
    st.stop()

account_id = st.selectbox("Account", account_ids)

stats = get_account_stats(account_id)

if not stats:
    st.info(f"No data for {account_id} yet.")
    st.stop()

col1, col2, col3, col4 = st.columns(4)
col1.metric("Equity", f"${stats['equity']:,.2f}")
col2.metric("Cash", f"${stats['cash']:,.2f}")
col3.metric("Trades", stats['total_trades'])
col4.metric("Signals", stats['total_signals'])

st.markdown("---")

st.subheader("Equity Curve")
history = get_account_history(account_id, 500)
if not history.empty:
    history['timestamp'] = pd.to_datetime(history['timestamp'], format='ISO8601')
    history = history.sort_values('timestamp')

    fig = go.Figure()
    fig.add_trace(go.Scatter(
        x=history['timestamp'], y=history['equity'],
        mode='lines', name='Equity',
        line=dict(color='#00d4aa', width=2),
        fill='tozeroy', fillcolor='rgba(0, 212, 170, 0.1)'
    ))
    fig.update_layout(
        xaxis_title="Time", yaxis_title="Equity ($)",
        hovermode='x unified', height=300,
        margin=dict(l=0, r=0, t=10, b=0),
    )
    st.plotly_chart(fig, use_container_width=True)
else:
    st.info("No equity history yet")

st.markdown("---")

col1, col2 = st.columns(2)

with col1:
    st.subheader("Recent Trades")
    trades_df = get_trades(account_id=account_id, limit=10)
    if not trades_df.empty:
        trades_df['value'] = trades_df['quantity'] * trades_df['price']
        st.dataframe(
            trades_df[['symbol', 'side', 'quantity', 'price', 'value', 'timestamp']],
            hide_index=True, use_container_width=True,
            column_config={
                'symbol': 'Symbol', 'side': 'Side',
                'quantity': st.column_config.NumberColumn('Qty', format='%.2f'),
                'price': st.column_config.NumberColumn('Price', format='$%.2f'),
                'value': st.column_config.NumberColumn('Value', format='$%.2f'),
                'timestamp': 'Time',
            }
        )
    else:
        st.info("No trades yet")

with col2:
    st.subheader("Recent Signals")
    signals_df = get_signals(account_id=account_id, limit=10)
    if not signals_df.empty:
        st.dataframe(
            signals_df[['symbol', 'signal_type', 'strength', 'strategy', 'timestamp']],
            hide_index=True, use_container_width=True,
            column_config={
                'symbol': 'Symbol', 'signal_type': 'Signal',
                'strength': st.column_config.NumberColumn('Strength', format='%.2f'),
                'strategy': 'Strategy', 'timestamp': 'Time',
            }
        )
    else:
        st.info("No signals yet")
