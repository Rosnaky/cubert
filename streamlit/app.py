import streamlit as st
import sqlite3
from datetime import datetime

st.set_page_config(
    page_title="Cubert Trading Bot",
    page_icon="🤖",
    layout="wide",
    initial_sidebar_state="expanded"
)

st.title("🤖 Cubert Trading Bot Dashboard")
st.markdown("Real-time monitoring for your algorithmic trading system")

# Sidebar
st.sidebar.title("Navigation")
st.sidebar.markdown("---")

# Connection status
try:
    conn = sqlite3.connect('../trading.db')
    st.sidebar.success("✅ Connected to database")
    conn.close()
except Exception as e:
    st.sidebar.error(f"❌ Database error: {e}")

st.sidebar.markdown("---")
st.sidebar.markdown(f"**Last Updated:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")

# Main page content
col1, col2, col3 = st.columns(3)

with col1:
    st.metric("Status", "Running", "Active")

with col2:
    st.metric("Strategies", "1", "Momentum")

with col3:
    st.metric("Symbols", "3", "AAPL, MSFT, TSLA")

st.markdown("---")
st.info("👈 Select a page from the sidebar to view detailed information")
