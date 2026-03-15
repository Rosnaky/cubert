import streamlit as st
import sys
sys.path.append('..')
from utils.db import get_positions, get_active_account_ids
import plotly.graph_objects as go

st.title("Positions")

account_ids = get_active_account_ids()

if not account_ids:
    st.info("No active accounts")
    st.stop()

account_id = st.selectbox("Account", account_ids)
positions_df = get_positions(account_id)

if not positions_df.empty:
    # Calculate metrics
    positions_df['market_value'] = positions_df['quantity'] * positions_df['current_price']
    positions_df['cost_basis'] = positions_df['quantity'] * positions_df['avg_entry_price']
    positions_df['pnl'] = positions_df['market_value'] - positions_df['cost_basis']
    positions_df['pnl_pct'] = (positions_df['pnl'] / positions_df['cost_basis']) * 100
    
    # Summary metrics
    total_value = positions_df['market_value'].sum()
    total_pnl = positions_df['pnl'].sum()
    
    col1, col2, col3 = st.columns(3)
    
    with col1:
        st.metric("Open Positions", len(positions_df))
    
    with col2:
        st.metric("Total Value", f"${total_value:,.2f}")
    
    with col3:
        st.metric("Total P&L", f"${total_pnl:,.2f}", delta=f"{(total_pnl/positions_df['cost_basis'].sum())*100:.2f}%")
    
    st.markdown("---")
    
    # Position details
    st.subheader("Position Details")
    
    for _, pos in positions_df.iterrows():
        with st.expander(f"{pos['symbol']} - {pos['quantity']:.0f} shares"):
            col1, col2, col3, col4 = st.columns(4)
            
            with col1:
                st.metric("Avg Entry", f"${pos['avg_entry_price']:.2f}")
            
            with col2:
                st.metric("Current Price", f"${pos['current_price']:.2f}")
            
            with col3:
                st.metric("P&L", f"${pos['pnl']:.2f}")
            
            with col4:
                st.metric("P&L %", f"{pos['pnl_pct']:.2f}%")
    
    st.markdown("---")
    
    # Portfolio allocation
    st.subheader("Portfolio Allocation")
    fig = go.Figure(data=[go.Pie(
        labels=positions_df['symbol'],
        values=positions_df['market_value'],
        hole=0.3
    )])
    fig.update_layout(title_text="Market Value by Symbol")
    st.plotly_chart(fig, use_container_width=True)
    
else:
    st.info("No open positions")
    