# Cubert

Algorithmic trading engine in Rust with CUDA math kernels.

The engine polls bars from Alpaca. Each account runs its own strategies. A risk manager
sizes the resulting orders and a paper or live broker executes them. A Streamlit interface allows for reconfiguration of strategies while the engine runs.

The math runs on the GPU including: rolling z-score, Ornstein-Uhlenbeck estimation
and an augmented Dickey-Fuller test. Those kernels come from
[gem-blockset](https://github.com/Rosnaky/gem-blockset), a CUDA library that ships as a git
submodule and links over FFI.

```
Alpaca data ─► Engine tick ─► Strategy.on_bar ─► RiskManager ─► Broker.submit_order
                    │                                               │
                    └──────────────► SQLite (cubert.db) ◄───────────┘
                                          ▲
                                          │
                                     Streamlit
```

## Requirements

- Rust, 2024 edition
- CUDA toolkit with `nvcc` and a GPU. [build.rs](build.rs) compiles the submodule for
  `CMAKE_CUDA_ARCHITECTURES=75`; edit it for other architectures.
- CMake, a C++ toolchain, libclang (for `bindgen`)
- Python 3
- Alpaca account and API keys

The build looks for CUDA in `CUDA_PATH`, then `CUDA_HOME`, then `/opt/cuda`.

## Build

```bash
git clone --recurse-submodules https://github.com/Rosnaky/cubert.git
cd cubert

# Already cloned without --recurse-submodules:
git submodule update --init --recursive

cargo build --release
```

## Configuration

`.env`:

```
ALPACA_API_KEY=your_key
ALPACA_API_SECRET=your_secret
```

`config.toml`:

```toml
[broker]
name = "cubert"
api_endpoint = "https://paper-api.alpaca.markets"
data_endpoint = "https://data.alpaca.markets"
paper = true

[data]
symbols = ["AAPL", "TSLA", "NVDA", "TSM"]
timeframe = "1Min"

[risk]
max_position_pct = 0.20
max_drawdown_pct = 0.25
max_daily_trades = 25
# Optional
max_total_exposure = 0.8
max_loss_per_trade = 0.02

[logging]
level = "info" # debug | info | warn | error
file = "logs/trades.log"

[storage]
db_url = "sqlite:cubert.db"
broker_db_url = "sqlite:broker.db"
```

`broker.paper = true` routes orders to `PaperAlpacaBroker`, which simulates fills at the
latest Alpaca close and keeps balances in a per-account SQLite file. `false` sends real
orders to `api_endpoint` through `AlpacaApiBroker`.

## Running

```bash
python -m venv venv
source venv/bin/activate
pip install -r streamlit/requirements.txt

streamlit run streamlit/app.py
```

1. **Strategies** page: create a strategy (name + type + params).
2. **Accounts** page: create an account with a starting cash balance.
3. **Accounts** page: assign the strategy to the account with a symbol list.
4. Start the engine: `cargo run --release`.

```bash
cargo run --release
tail -f logs/trades.log
```

On startup the engine loads the config and migrates `cubert.db`. It then queries
`SELECT DISTINCT account_id FROM account_strategies WHERE enabled = TRUE`, builds one broker
per account and warms every strategy with historical bars. Then it ticks every 60 seconds
([main.rs:115](src/main.rs#L115)).

Each tick:

1. Re-reads strategy assignments. Any strategy whose name, symbols or params changed gets
   rebuilt. A rebuilt strategy loses its price buffer and re-warms from live bars only.
2. Fetches latest bars for the union of all assigned symbols.
3. Feeds each bar to every strategy subscribed to that symbol.
4. Passes buy/sell signals to the risk manager and submits whatever it accepts.
5. Writes an equity snapshot and upserts positions per account.

## Streamlit interface

`streamlit/app.py` is the entry point. The sidebar lists the pages below. Most pages start
with an account selector. Pages that read live data also give you a `Refresh` button, which
simply reruns the SQLite queries because nothing here is cached.

### Strategies

Create and edit strategy definitions. A strategy holds a name, a type and a JSON param
blob in the `strategies` table. It belongs to no account until you assign it.

- **Create Strategy**: pick a type, name it and fill in the params. Every input carries a
  caption explaining what it does. The page stores params as `{"type": "<Type>", ...}` to
  match the serde tag on [StrategyParams](src/strategy/mod.rs).
- **Existing Strategies**: each expander shows the strategy ID and its editable params.
  `Save Changes` wakes up only after you change a value. `Delete` drops the definition.

Editing a strategy hits every account that uses it.

Momentum params:

| Param | Default | Effect |
| --- | --- | --- |
| `lookback_period` | 20 | Bars back to compare the current close against |
| `threshold` | 0.02 | Return needed to fire a signal; higher means fewer trades |
| `startup_lookback` | `1Hour` | Bar timeframe used for warmup |
| `startup_bar_limit` | 50 | Warmup bars fetched |

Mean reversion params:

| Param | Default | Effect |
| --- | --- | --- |
| `window` | 60 | Rolling window for z-score and OU estimation |
| `zscore_entry` | 2.0 | Z-score below `-entry` opens a long |
| `zscore_exit` | 0.5 | Exit when volatility-normalized expected move drops below this |
| `min_half_life` | 2.0 | Reject symbols reverting faster than this (noise) |
| `max_half_life` | 50.0 | Reject symbols reverting slower than this |
| `adf_lags` | 1 | Lags in the ADF regression |
| `recompute_interval` | 5 | Bars between GPU recomputes |
| `max_buffer` | 500 | Max prices retained per symbol |
| `startup_lookback` | `1Hour` | Warmup timeframe |
| `startup_bar_limit` | 500 | Warmup bars; must exceed `window` or nothing trades |

A symbol produces signals only when ADF rejects a unit root at the 10% level and the
half-life `ln(2)/θ` lands inside the min/max band. Two factors then scale entry strength:
ADF confidence (1%/5%/10% → 1.0/0.75/0.5) and realized volatility against a 2% baseline.
The result clamps to 1.0. See [mean_reversion.rs](src/strategy/mean_reversion.rs).

### Accounts

Create accounts here and bind strategies to symbols.

- **Active Accounts**: one expander per account. Each shows equity, trade count, signal
  count and every strategy assigned to it. Each assignment gets an `Enabled` toggle that
  writes immediately, a symbol multiselect, a text field for adding symbols the dropdown
  does not know yet, plus `Save Changes` and `Remove Strategy`.
- **Create Account**: account ID and starting cash. The ID becomes the `account_id` in
  every table and names the paper broker DB file.
- **Assign Strategy to Account**: pick an account and a strategy, then list at least one
  symbol.

Two accounts can run the same strategy over different symbols. The engine builds a separate
instance with its own price buffers for each.

`Delete Account` clears that account from six tables: `account`, `account_strategies`,
`signals`, `trades`, `account_snapshots` and `positions`. It leaves
`broker_<account_id>.db` on disk, so a new account reusing the old ID inherits the old
simulated cash and positions. Delete that file yourself for a clean slate.

Starting cash writes to `cubert.db` only. The paper broker seeds its own DB with $100,000
on first run no matter what you typed ([main.rs:74](src/main.rs#L74)). Change that call to
start paper accounts anywhere else.

## Adding a strategy

1. Implement [Strategy](src/strategy/mod.rs) in `src/strategy/`.
2. Add a variant to `StrategyParams` and a match arm in `create_strategy`.
3. Add a `StrategyDefinition` subclass in `streamlit/strategies/` and register it in
   `_ALL_STRATEGIES`.

`strategy_type()` on the Python side must match the serde tag of the Rust enum variant
exactly. Otherwise the engine fails to deserialize `params_json`.

## Tests

```bash
cargo test
```

The suites in [tests/](tests/) hit storage, brokers, risk, strategies and a full trade
cycle, all against in-memory SQLite. The CUDA kernels carry their own tests under
[dependencies/gem-blockset/tests/](dependencies/gem-blockset/tests/).

[CI](.github/workflows/ci.yml) runs `cargo fmt --check`, then `cargo clippy -D warnings`,
then `cargo test --all-features`, then a release build against CUDA 12.8.

## License

MIT, see [LICENSE](LICENSE).
