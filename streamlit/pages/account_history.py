import streamlit as st
import pandas as pd
import sys
sys.path.append('..')
from utils.db import get_account_history, get_active_account_ids
import plotly.graph_objects as go

st.set_page_config(page_title="Account History", layout="wide")
st.title("Account History")

account_ids = get_active_account_ids()

if not account_ids:
    st.info("No active accounts found")
    st.stop()

view_mode = st.radio("View", ["Single Account", "All Accounts"], horizontal=True)

if view_mode == "Single Account":
    account_id = st.selectbox("Account", account_ids)
    account_df = get_account_history(account_id=account_id, limit=500)

    if account_df.empty:
        st.info(f"No history for {account_id}")
        st.stop()

    account_df = account_df.sort_values('timestamp')
    account_df['timestamp'] = pd.to_datetime(account_df['timestamp'], format='ISO8601')
    latest = account_df.iloc[-1]

    col1, col2, col3 = st.columns(3)
    col1.metric("Current Equity", f"${latest['equity']:,.2f}")
    col2.metric("Cash", f"${latest['cash']:,.2f}")
    col3.metric("Buying Power", f"${latest['buying_power']:,.2f}")

    st.markdown("---")
    st.subheader("Equity Curve")

    fig = go.Figure()
    fig.add_trace(go.Scatter(
        x=account_df['timestamp'], y=account_df['equity'],
        mode='lines', name='Equity', line=dict(color='green', width=2)
    ))
    fig.add_trace(go.Scatter(
        x=account_df['timestamp'], y=account_df['cash'],
        mode='lines', name='Cash', line=dict(color='blue', width=2, dash='dash')
    ))
    fig.update_layout(xaxis_title="Date", yaxis_title="Value ($)", hovermode='x unified')
    st.plotly_chart(fig, use_container_width=True)

    st.markdown("---")
    st.subheader("Performance")

    initial_equity = account_df.iloc[0]['equity']
    final_equity = latest['equity']
    total_return = ((final_equity - initial_equity) / initial_equity) * 100 if initial_equity > 0 else 0

    col1, col2, col3 = st.columns(3)
    col1.metric("Initial Equity", f"${initial_equity:,.2f}")
    col2.metric("Final Equity", f"${final_equity:,.2f}")
    col3.metric("Total Return", f"{total_return:.2f}%")

else:
    st.subheader("All Accounts")

    rows = []
    for aid in account_ids:
        df = get_account_history(account_id=aid, limit=500)
        if df.empty:
            continue
        df = df.sort_values('timestamp')
        initial = df.iloc[0]['equity']
        final = df.iloc[-1]['equity']
        ret = ((final - initial) / initial) * 100 if initial > 0 else 0
        rows.append({
            "Account": aid,
            "Equity": f"${final:,.2f}",
            "Cash": f"${df.iloc[-1]['cash']:,.2f}",
            "Return": f"{ret:.2f}%",
        })

    if rows:
        st.dataframe(pd.DataFrame(rows), use_container_width=True, hide_index=True)

    st.markdown("---")
    st.subheader("Equity Curves")

    fig = go.Figure()
    for aid in account_ids:
        df = get_account_history(account_id=aid, limit=500)
        if df.empty:
            continue
        df = df.sort_values('timestamp')
        df['timestamp'] = pd.to_datetime(df['timestamp'], format='ISO8601')
        fig.add_trace(go.Scatter(
            x=df['timestamp'], y=df['equity'],
            mode='lines', name=aid, line=dict(width=2)
        ))

    fig.update_layout(xaxis_title="Date", yaxis_title="Equity ($)", hovermode='x unified')
    st.plotly_chart(fig, use_container_width=True)
