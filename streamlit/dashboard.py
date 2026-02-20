import sqlite3
import pandas as pd
import streamlit as st

conn = sqlite3.connect("cubert.db")

trades = pd.read_sql("SELECT * FROM trades ORDER BY timestamp DESC", conn)
st.dataframe(trades)

account = pd.read_sql("SELECT * FROM account_snapshots ORDER BY timestamp DESC", conn)
st.line_chart(account.set_index("timestamp")["equity"])
