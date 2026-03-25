import sqlite3
import json
import uuid
from datetime import datetime
import pandas as pd

ENGINE_DB = "cubert.db"


def get_engine_connection():
    return sqlite3.connect(ENGINE_DB)


# ============ Accounts ============

def get_active_account_ids():
    conn = get_engine_connection()
    df = pd.read_sql_query(
        "SELECT DISTINCT account_id FROM account_strategies WHERE enabled = TRUE",
        conn
    )
    conn.close()
    return df['account_id'].tolist()


def get_all_account_ids():
    conn = get_engine_connection()
    df = pd.read_sql_query("SELECT id FROM account", conn)
    conn.close()
    return df['id'].tolist()


def create_account(account_id, starting_cash=100000.0):
    conn = get_engine_connection()
    now = datetime.utcnow().isoformat()

    conn.execute(
        """INSERT OR IGNORE INTO account (id, cash, created_at, updated_at)
           VALUES (?, ?, ?, ?)""",
        (account_id, starting_cash, now, now)
    )

    conn.execute(
        """INSERT INTO account_snapshots (id, account_id, equity, cash, buying_power, timestamp)
           VALUES (?, ?, ?, ?, ?, ?)""",
        (str(uuid.uuid4()), account_id, starting_cash, starting_cash, starting_cash, now)
    )

    conn.commit()
    conn.close()

def delete_account(account_id):
    conn = get_engine_connection()
    conn.execute("DELETE FROM account_strategies WHERE account_id = ?", (account_id,))
    conn.execute("DELETE FROM account WHERE id = ?", (account_id,))
    conn.execute("DELETE FROM signals WHERE account_id = ?", (account_id,))
    conn.execute("DELETE FROM trades WHERE account_id = ?", (account_id,))
    conn.execute("DELETE FROM account_snapshots WHERE account_id = ?", (account_id,))
    conn.execute("DELETE FROM positions WHERE account_id = ?", (account_id,))
    conn.commit()
    conn.close()

# ============ Account History ============

def get_account_history(account_id, limit=1000):
    conn = get_engine_connection()
    df = pd.read_sql_query(
        "SELECT * FROM account_snapshots WHERE account_id = ? ORDER BY timestamp ASC LIMIT ?",
        conn,
        params=(account_id, limit)
    )
    conn.close()
    return df


def get_latest_snapshot(account_id):
    conn = get_engine_connection()
    df = pd.read_sql_query(
        "SELECT * FROM account_snapshots WHERE account_id = ? ORDER BY timestamp DESC LIMIT 1",
        conn,
        params=(account_id,)
    )
    conn.close()
    if df.empty:
        return None
    return df.iloc[0].to_dict()


# ============ Trades ============

def get_trades(account_id=None, limit=100):
    conn = get_engine_connection()
    if account_id:
        df = pd.read_sql_query(
            "SELECT * FROM trades WHERE account_id = ? ORDER BY timestamp DESC LIMIT ?",
            conn,
            params=(account_id, limit)
        )
    else:
        df = pd.read_sql_query(
            "SELECT * FROM trades ORDER BY timestamp DESC LIMIT ?",
            conn,
            params=(limit,)
        )
    conn.close()
    return df


# ============ Signals ============

def get_signals(account_id=None, limit=100):
    conn = get_engine_connection()
    if account_id:
        df = pd.read_sql_query(
            "SELECT * FROM signals WHERE account_id = ? ORDER BY timestamp DESC LIMIT ?",
            conn,
            params=(account_id, limit)
        )
    else:
        df = pd.read_sql_query(
            "SELECT * FROM signals ORDER BY timestamp DESC LIMIT ?",
            conn,
            params=(limit,)
        )
    conn.close()
    return df


# ============ Strategies ============

def get_strategies():
    conn = get_engine_connection()
    df = pd.read_sql_query("SELECT * FROM strategies", conn)
    conn.close()
    return df


def get_strategies_for_account(account_id):
    conn = get_engine_connection()
    df = pd.read_sql_query(
        """SELECT s.*, a.symbols_json, a.enabled, a.assigned_at
           FROM strategies s
           JOIN account_strategies a ON s.id = a.strategy_id
           WHERE a.account_id = ?""",
        conn,
        params=(account_id,)
    )
    conn.close()
    return df


def insert_strategy(name, strategy_type, params_json):
    conn = get_engine_connection()
    strategy_id = str(uuid.uuid4())
    now = datetime.utcnow().isoformat()
    conn.execute(
        "INSERT INTO strategies (id, name, strategy_type, params_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        (strategy_id, name, strategy_type, json.dumps(params_json), now, now)
    )
    conn.commit()
    conn.close()
    return strategy_id


def update_strategy(strategy_id, name, strategy_type, params_json):
    conn = get_engine_connection()
    now = datetime.utcnow().isoformat()
    conn.execute(
        "UPDATE strategies SET name = ?, strategy_type = ?, params_json = ?, updated_at = ? WHERE id = ?",
        (name, strategy_type, json.dumps(params_json), now, strategy_id)
    )
    conn.commit()
    conn.close()


def delete_strategy(strategy_id):
    conn = get_engine_connection()
    conn.execute("DELETE FROM account_strategies WHERE strategy_id = ?", (strategy_id,))
    conn.execute("DELETE FROM strategies WHERE id = ?", (strategy_id,))
    conn.commit()
    conn.close()

# ============ Positions ============

def get_positions(account_id):
    conn = get_engine_connection()
    df = pd.read_sql_query(
        "SELECT * FROM positions WHERE account_id = ? AND quantity != 0",
        conn,
        params=(account_id,)
    )
    conn.close()
    return df

# ============ Account Strategies ============

def assign_strategy_to_account(account_id, strategy_id, symbols):
    conn = get_engine_connection()
    conn.execute(
        "INSERT OR REPLACE INTO account_strategies (account_id, strategy_id, symbols_json, enabled) VALUES (?, ?, ?, TRUE)",
        (account_id, strategy_id, json.dumps(symbols))
    )
    conn.commit()
    conn.close()


def unassign_strategy_from_account(account_id, strategy_id):
    conn = get_engine_connection()
    conn.execute(
        "DELETE FROM account_strategies WHERE account_id = ? AND strategy_id = ?",
        (account_id, strategy_id)
    )
    conn.commit()
    conn.close()

def update_assignment_symbols(account_id, strategy_id, symbols):
    conn = get_engine_connection()
    conn.execute(
        "UPDATE account_strategies SET symbols_json = ? WHERE account_id = ? AND strategy_id = ?",
        (json.dumps(symbols), account_id, strategy_id)
    )
    conn.commit()
    conn.close()


def toggle_strategy_enabled(account_id, strategy_id, enabled):
    conn = get_engine_connection()
    conn.execute(
        "UPDATE account_strategies SET enabled = ? WHERE account_id = ? AND strategy_id = ?",
        (enabled, account_id, strategy_id)
    )
    conn.commit()
    conn.close()


def get_all_assignments():
    conn = get_engine_connection()
    df = pd.read_sql_query(
        """SELECT a.account_id, a.strategy_id, a.symbols_json, a.enabled, a.assigned_at, s.name, s.strategy_type
           FROM account_strategies a
           JOIN strategies s ON s.id = a.strategy_id""",
        conn
    )
    conn.close()
    return df


# ============ Stats ============

def get_account_stats(account_id):
    latest = get_latest_snapshot(account_id)
    if not latest:
        return None

    conn = get_engine_connection()
    cursor = conn.cursor()

    cursor.execute("SELECT COUNT(*) FROM trades WHERE account_id = ?", (account_id,))
    total_trades = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM signals WHERE account_id = ?", (account_id,))
    total_signals = cursor.fetchone()[0]

    conn.close()

    return {
        'account_id': account_id,
        'equity': latest['equity'],
        'cash': latest['cash'],
        'buying_power': latest['buying_power'],
        'total_trades': total_trades,
        'total_signals': total_signals,
    }


def get_strategy_performance(account_id=None):
    conn = get_engine_connection()
    if account_id:
        df = pd.read_sql_query(
            """SELECT strategy, COUNT(*) as trade_count,
                SUM(CASE WHEN side = 'buy' THEN quantity * price ELSE 0 END) as total_bought,
                SUM(CASE WHEN side = 'sell' THEN quantity * price ELSE 0 END) as total_sold
            FROM trades WHERE strategy IS NOT NULL AND account_id = ?
            GROUP BY strategy""",
            conn,
            params=(account_id,)
        )
    else:
        df = pd.read_sql_query(
            """SELECT account_id, strategy, COUNT(*) as trade_count,
                SUM(CASE WHEN side = 'buy' THEN quantity * price ELSE 0 END) as total_bought,
                SUM(CASE WHEN side = 'sell' THEN quantity * price ELSE 0 END) as total_sold
            FROM trades WHERE strategy IS NOT NULL
            GROUP BY account_id, strategy""",
            conn
        )
    conn.close()
    return df


def get_signal_stats(account_id=None):
    conn = get_engine_connection()
    if account_id:
        df = pd.read_sql_query(
            """SELECT strategy, signal_type, COUNT(*) as count, AVG(strength) as avg_strength
            FROM signals WHERE account_id = ?
            GROUP BY strategy, signal_type""",
            conn,
            params=(account_id,)
        )
    else:
        df = pd.read_sql_query(
            """SELECT account_id, strategy, signal_type, COUNT(*) as count, AVG(strength) as avg_strength
            FROM signals
            GROUP BY account_id, strategy, signal_type""",
            conn
        )
    conn.close()
    return df
