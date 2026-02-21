//! SQLite storage implementation.

use chrono::Utc;
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use uuid::Uuid;

use super::models::{DbAccount, DbAccountSnapshot, DbPosition, DbSignal, DbTrade};
use crate::types::{Account, Position};

#[derive(Debug)]
pub enum StorageError {
    Connection(String),
    Query(String),
    NotFound,
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Connection(msg) => write!(f, "Connection error: {}", msg),
            StorageError::Query(msg) => write!(f, "Query error: {}", msg),
            StorageError::NotFound => write!(f, "Not found"),
        }
    }
}

impl std::error::Error for StorageError {}

pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), StorageError> {
        // Trades table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trades (
                id TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                side TEXT NOT NULL,
                quantity REAL NOT NULL,
                price REAL NOT NULL,
                timestamp TEXT NOT NULL,
                order_id TEXT,
                strategy TEXT
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        // Positions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS positions (
                id TEXT PRIMARY KEY,
                symbol TEXT NOT NULL UNIQUE,
                quantity REAL NOT NULL,
                avg_entry_price REAL NOT NULL,
                current_price REAL NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        // Account snapshots table (historical)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS account_snapshots (
                id TEXT PRIMARY KEY,
                equity REAL NOT NULL,
                cash REAL NOT NULL,
                buying_power REAL NOT NULL,
                timestamp TEXT NOT NULL
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        // Signals table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS signals (
                id TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                signal_type TEXT NOT NULL,
                strength REAL NOT NULL,
                strategy TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        // Account table (current state - single row)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS account (
                id TEXT PRIMARY KEY,
                cash REAL NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        // Indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_timestamp ON trades(timestamp DESC)")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_symbol ON trades(symbol)")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    // ============ Account ============

    /// Initialize account with starting cash (only if not exists)
    pub async fn init_account(&self, starting_cash: f64) -> Result<(), StorageError> {
        let existing = sqlx::query("SELECT id FROM account LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        if existing.is_none() {
            let id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();

            sqlx::query(
                r#"
                INSERT INTO account (id, cash, created_at, updated_at)
                VALUES (?, ?, ?, ?)
            "#,
            )
            .bind(&id)
            .bind(starting_cash)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;
        }

        Ok(())
    }

    /// Get current account state (cash + positions = equity)
    pub async fn get_account(&self) -> Result<Account, StorageError> {
        let row = sqlx::query("SELECT cash FROM account LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?
            .ok_or(StorageError::NotFound)?;

        let cash: f64 = row.get("cash");

        // Calculate equity from positions
        let positions = self.get_positions().await?;
        let positions_value: f64 = positions.iter().map(|p| p.quantity * p.current_price).sum();

        let equity = cash + positions_value;

        Ok(Account {
            equity,
            cash,
            buying_power: cash, // Simplified: buying power = cash
        })
    }

    /// Update account cash
    pub async fn update_cash(&self, new_cash: f64) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();

        sqlx::query("UPDATE account SET cash = ?, updated_at = ?")
            .bind(new_cash)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    /// Deduct cash (for buying)
    pub async fn deduct_cash(&self, amount: f64) -> Result<f64, StorageError> {
        let account = self.get_account().await?;
        let new_cash = account.cash - amount;

        if new_cash < 0.0 {
            return Err(StorageError::Query("Insufficient funds".to_string()));
        }

        self.update_cash(new_cash).await?;
        Ok(new_cash)
    }

    /// Add cash (for selling)
    pub async fn add_cash(&self, amount: f64) -> Result<f64, StorageError> {
        let account = self.get_account().await?;
        let new_cash = account.cash + amount;
        self.update_cash(new_cash).await?;
        Ok(new_cash)
    }

    // ============ Positions ============

    pub async fn get_positions(&self) -> Result<Vec<Position>, StorageError> {
        let rows = sqlx::query_as::<_, DbPosition>("SELECT * FROM positions WHERE quantity != 0")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|p| Position {
                symbol: p.symbol,
                quantity: p.quantity,
                avg_entry_price: p.avg_entry_price,
                current_price: p.current_price,
            })
            .collect())
    }

    pub async fn get_position(&self, symbol: &str) -> Result<Option<Position>, StorageError> {
        let row = sqlx::query_as::<_, DbPosition>(
            "SELECT * FROM positions WHERE symbol = ? AND quantity != 0",
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(row.map(|p| Position {
            symbol: p.symbol,
            quantity: p.quantity,
            avg_entry_price: p.avg_entry_price,
            current_price: p.current_price,
        }))
    }

    pub async fn upsert_position(
        &self,
        symbol: &str,
        quantity: f64,
        avg_entry_price: f64,
        current_price: f64,
    ) -> Result<(), StorageError> {
        let id = Uuid::new_v4().to_string();
        let updated_at = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO positions (id, symbol, quantity, avg_entry_price, current_price, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(symbol) DO UPDATE SET
                quantity = excluded.quantity,
                avg_entry_price = excluded.avg_entry_price,
                current_price = excluded.current_price,
                updated_at = excluded.updated_at
        "#,
        )
        .bind(&id)
        .bind(symbol)
        .bind(quantity)
        .bind(avg_entry_price)
        .bind(current_price)
        .bind(&updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    pub async fn update_position_price(
        &self,
        symbol: &str,
        current_price: f64,
    ) -> Result<(), StorageError> {
        let updated_at = Utc::now().to_rfc3339();

        sqlx::query("UPDATE positions SET current_price = ?, updated_at = ? WHERE symbol = ?")
            .bind(current_price)
            .bind(&updated_at)
            .bind(symbol)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_position(&self, symbol: &str) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM positions WHERE symbol = ?")
            .bind(symbol)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    // ============ Trades ============

    pub async fn insert_trade(
        &self,
        symbol: &str,
        side: &str,
        quantity: f64,
        price: f64,
        order_id: Option<&str>,
        strategy: Option<&str>,
    ) -> Result<String, StorageError> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO trades (id, symbol, side, quantity, price, timestamp, order_id, strategy)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&id)
        .bind(symbol)
        .bind(side)
        .bind(quantity)
        .bind(price)
        .bind(&timestamp)
        .bind(order_id)
        .bind(strategy)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(id)
    }

    pub async fn get_trades(&self, limit: i32) -> Result<Vec<DbTrade>, StorageError> {
        sqlx::query_as::<_, DbTrade>("SELECT * FROM trades ORDER BY timestamp DESC LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))
    }

    pub async fn get_trades_by_symbol(
        &self,
        symbol: &str,
        limit: i32,
    ) -> Result<Vec<DbTrade>, StorageError> {
        sqlx::query_as::<_, DbTrade>(
            "SELECT * FROM trades WHERE symbol = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(symbol)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))
    }

    // ============ Account Snapshots ============

    pub async fn insert_account_snapshot(
        &self,
        equity: f64,
        cash: f64,
        buying_power: f64,
    ) -> Result<String, StorageError> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO account_snapshots (id, equity, cash, buying_power, timestamp)
            VALUES (?, ?, ?, ?, ?)
        "#,
        )
        .bind(&id)
        .bind(equity)
        .bind(cash)
        .bind(buying_power)
        .bind(&timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(id)
    }

    pub async fn get_account_history(
        &self,
        limit: i32,
    ) -> Result<Vec<DbAccountSnapshot>, StorageError> {
        sqlx::query_as::<_, DbAccountSnapshot>(
            "SELECT * FROM account_snapshots ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))
    }

    // ============ Signals ============

    pub async fn insert_signal(
        &self,
        symbol: &str,
        signal_type: &str,
        strength: f64,
        strategy: &str,
    ) -> Result<String, StorageError> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO signals (id, symbol, signal_type, strength, strategy, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&id)
        .bind(symbol)
        .bind(signal_type)
        .bind(strength)
        .bind(strategy)
        .bind(&timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(id)
    }

    pub async fn get_signals(&self, limit: i32) -> Result<Vec<DbSignal>, StorageError> {
        sqlx::query_as::<_, DbSignal>("SELECT * FROM signals ORDER BY timestamp DESC LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))
    }
}
