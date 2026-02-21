import streamlit as st
import sys
sys.path.append('..')
from utils.db import get_account_history
import plotly.graph_objects as go
import pandas as pd

st.title("💰 Account History")

account_df = get_account_history(limit=500)

if not account_df.empty:
    # Sort by timestamp
    account_df = account_df.sort_values('timestamp')
    account_df['timestamp'] = pd.to_datetime(account_df['timestamp'])
    
    # Current stats
    latest = account_df.iloc[-1]
    
    col1, col2, col3 = st.columns(3)
    
    with col1:
        st.metric("Current Equity", f"${latest['equity']:,.2f}")
    
    with col2:
        st.metric("Cash", f"${latest['cash']:,.2f}")
    
    with col3:
        st.metric("Buying Power", f"${latest['buying_power']:,.2f}")
    
    st.markdown("---")
    
    # Equity curve
    st.subheader("Equity Curve")
    
    fig = go.Figure()
    
    fig.add_trace(go.Scatter(
        x=account_df['timestamp'],
        y=account_df['equity'],
        mode='lines',
        name='Equity',
        line=dict(color='green', width=2)
    ))
    
    fig.add_trace(go.Scatter(
        x=account_df['timestamp'],
        y=account_df['cash'],
        mode='lines',
        name='Cash',
        line=dict(color='blue', width=2, dash='dash')
    ))
    
    fig.update_layout(
        xaxis_title="Date",
        yaxis_title="Value ($)",
        hovermode='x unified'
    )
    
    st.plotly_chart(fig, use_container_width=True)
    
    st.markdown("---")
    
    # Stats
    st.subheader("Performance Statistics")
    
    initial_equity = account_df.iloc[0]['equity']
    final_equity = latest['equity']
    total_return = ((final_equity - initial_equity) / initial_equity) * 100
    
    col1, col2, col3 = st.columns(3)
    
    with col1:
        st.metric("Initial Equity", f"${initial_equity:,.2f}")
    
    with col2:
        st.metric("Final Equity", f"${final_equity:,.2f}")
    
    with col3:
        st.metric("Total Return", f"{total_return:.2f}%")
    
else:
    st.info("No account history yet")
    