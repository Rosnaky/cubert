import sqlite3
import pandas as pd

DB_PATH = "cubert.db"

def get_connection():
    return sqlite3.connect(DB_PATH)

def get_signals(limit=100):
    conn = get_connection()
    query = """
    SELECT * FROM signals 
    ORDER BY timestamp DESC 
    LIMIT ?
    """
    df = pd.read_sql_query(query, conn, params=(limit,))
    conn.close()
    return df

def get_trades(limit=100):
    conn = get_connection()
    query = """
    SELECT * FROM trades 
    ORDER BY timestamp DESC 
    LIMIT ?
    """
    df = pd.read_sql_query(query, conn, params=(limit,))
    conn.close()
    return df

def get_positions():
    conn = get_connection()
    query = "SELECT * FROM positions WHERE quantity != 0"
    df = pd.read_sql_query(query, conn)
    conn.close()
    return df

def get_account_history(limit=100):
    conn = get_connection()
    query = """
    SELECT * FROM account_snapshots 
    ORDER BY timestamp DESC 
    LIMIT ?
    """
    df = pd.read_sql_query(query, conn, params=(limit,))
    conn.close()
    return df

def get_account_stats():
    conn = get_connection()
    cursor = conn.cursor()
    
    # Get latest account snapshot
    cursor.execute("SELECT equity, cash, buying_power FROM account_snapshots ORDER BY timestamp DESC LIMIT 1")
    account = cursor.fetchone()
    
    # Get total trades
    cursor.execute("SELECT COUNT(*) FROM trades")
    total_trades = cursor.fetchone()[0]
    
    # Get total signals
    cursor.execute("SELECT COUNT(*) FROM signals")
    total_signals = cursor.fetchone()[0]
    
    conn.close()
    
    return {
        'equity': account[0] if account else 0,
        'cash': account[1] if account else 0,
        'buying_power': account[2] if account else 0,
        'total_trades': total_trades,
        'total_signals': total_signals
    }
