import sqlite3
import pandas as pd

BROKER_DB = "broker.db"
ENGINE_DB = "cubert.db"


def get_broker_connection():
    return sqlite3.connect(BROKER_DB)


def get_engine_connection():
    return sqlite3.connect(ENGINE_DB)


# ============ Broker DB (source of truth) ============

def get_account():
    """Get current account state from broker"""
    conn = get_broker_connection()
    query = "SELECT cash FROM account LIMIT 1"
    df = pd.read_sql_query(query, conn)
    
    positions = get_positions()
    positions_value = (positions['quantity'] * positions['current_price']).sum() if not positions.empty else 0
    
    conn.close()
    
    cash = df['cash'].iloc[0] if not df.empty else 0
    return {
        'equity': cash + positions_value,
        'cash': cash,
        'buying_power': cash,
    }


def get_positions():
    """Get current positions from broker"""
    conn = get_broker_connection()
    query = "SELECT * FROM positions WHERE quantity != 0"
    df = pd.read_sql_query(query, conn)
    conn.close()
    return df


def get_broker_trades(limit=100):
    """Get executed trades from broker (source of truth)"""
    conn = get_broker_connection()
    query = """
    SELECT * FROM trades 
    ORDER BY timestamp DESC 
    LIMIT ?
    """
    df = pd.read_sql_query(query, conn, params=(limit,))
    conn.close()
    return df


def get_account_history(limit=1000):
    """Get account equity history from broker"""
    conn = get_broker_connection()
    query = """
    SELECT * FROM account_snapshots 
    ORDER BY timestamp ASC 
    LIMIT ?
    """
    df = pd.read_sql_query(query, conn, params=(limit,))
    conn.close()
    return df


# ============ Engine DB (logging/analytics) ============

def get_signals(limit=100):
    """Get signals from engine"""
    conn = get_engine_connection()
    query = """
    SELECT * FROM signals 
    ORDER BY timestamp DESC 
    LIMIT ?
    """
    df = pd.read_sql_query(query, conn, params=(limit,))
    conn.close()
    return df


def get_engine_trades(limit=100):
    """Get trades with strategy info from engine"""
    conn = get_engine_connection()
    query = """
    SELECT * FROM trades 
    ORDER BY timestamp DESC 
    LIMIT ?
    """
    df = pd.read_sql_query(query, conn, params=(limit,))
    conn.close()
    return df


def get_engine_snapshots(limit=100):
    """Get account snapshots from engine (for comparison)"""
    conn = get_engine_connection()
    query = """
    SELECT * FROM account_snapshots 
    ORDER BY timestamp DESC 
    LIMIT ?
    """
    df = pd.read_sql_query(query, conn, params=(limit,))
    conn.close()
    return df


# ============ Combined Stats ============

def get_account_stats():
    """Get combined stats from both databases"""
    account = get_account()
    positions = get_positions()
    
    # From engine
    engine_conn = get_engine_connection()
    cursor = engine_conn.cursor()
    
    cursor.execute("SELECT COUNT(*) FROM trades")
    total_trades = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(*) FROM signals")
    total_signals = cursor.fetchone()[0]
    
    engine_conn.close()
    
    # Calculate P&L
    if not positions.empty:
        positions['pnl'] = (positions['current_price'] - positions['avg_entry_price']) * positions['quantity']
        total_pnl = positions['pnl'].sum()
    else:
        total_pnl = 0
    
    return {
        'equity': account['equity'],
        'cash': account['cash'],
        'buying_power': account['buying_power'],
        'total_trades': total_trades,
        'total_signals': total_signals,
        'positions_count': len(positions),
        'unrealized_pnl': total_pnl,
    }


def get_strategy_performance():
    """Get performance by strategy from engine"""
    conn = get_engine_connection()
    query = """
    SELECT 
        strategy,
        COUNT(*) as trade_count,
        SUM(CASE WHEN side = 'buy' THEN quantity * price ELSE 0 END) as total_bought,
        SUM(CASE WHEN side = 'sell' THEN quantity * price ELSE 0 END) as total_sold
    FROM trades
    WHERE strategy IS NOT NULL
    GROUP BY strategy
    """
    df = pd.read_sql_query(query, conn)
    conn.close()
    return df


def get_signal_stats():
    """Get signal statistics from engine"""
    conn = get_engine_connection()
    query = """
    SELECT 
        strategy,
        signal_type,
        COUNT(*) as count,
        AVG(strength) as avg_strength
    FROM signals
    GROUP BY strategy, signal_type
    """
    df = pd.read_sql_query(query, conn)
    conn.close()
    return df
