import streamlit as st
import json
import sys
sys.path.append('..')
from utils.db import (
    create_account, delete_account, get_active_account_ids, get_all_account_ids, get_strategies,
    get_strategies_for_account, assign_strategy_to_account, toggle_strategy_enabled,
    unassign_strategy_from_account, update_assignment_symbols,
    get_account_stats, get_all_assignments,
)

st.set_page_config(page_title="Accounts", layout="wide")
st.title("Accounts")

if st.button("Refresh"):
    st.rerun()


def get_all_known_symbols():
    df = get_all_assignments()
    if df.empty:
        return []
    symbols = set()
    for _, row in df.iterrows():
        try:
            syms = json.loads(row['symbols_json'])
            symbols.update(syms)
        except (json.JSONDecodeError, TypeError):
            pass
    return sorted(symbols)


def symbol_picker(label, current_symbols, key_prefix):
    known = get_all_known_symbols()
    all_options = sorted(set(known + current_symbols))

    selected = st.multiselect(
        label,
        options=all_options,
        default=current_symbols,
        key=f"{key_prefix}_multi",
        placeholder="Select or type symbols...",
    )

    new_input = st.text_input(
        "Add new symbols (comma-separated)",
        key=f"{key_prefix}_new",
        placeholder="e.g. NVDA, AMD",
    )

    if new_input.strip():
        for s in new_input.split(","):
            s = s.strip().upper()
            if s and s not in selected:
                selected.append(s)

    return selected


# ============ Account Overview ============

st.subheader("Active Accounts")

account_ids = get_all_account_ids()
active_ids = get_active_account_ids()

if account_ids:
    for aid in account_ids:
        is_active = aid in active_ids
        status = "Active" if is_active else "Inactive"

        with st.expander(f"{aid} — {status}"):
            stats = get_account_stats(aid)
            if stats:
                col1, col2, col3 = st.columns(3)
                col1.metric("Equity", f"${stats['equity']:,.2f}")
                col2.metric("Trades", stats['total_trades'])
                col3.metric("Signals", stats['total_signals'])

            strategies_df = get_strategies_for_account(aid)
            if not strategies_df.empty:
                for _, row in strategies_df.iterrows():
                    symbols = json.loads(row['symbols_json']) if row['symbols_json'] else []
                    is_enabled = bool(row['enabled'])

                    col_name, col_toggle = st.columns([4, 1])
                    with col_name:
                        st.markdown(f"**{row['name']}** ({row['strategy_type']})")
                    with col_toggle:
                        new_enabled = st.toggle(
                            "Enabled",
                            value=is_enabled,
                            key=f"toggle_{aid}_{row['id']}",
                        )
                        if new_enabled != is_enabled:
                            toggle_strategy_enabled(aid, row['id'], new_enabled)
                            st.rerun()

                    updated_symbols = symbol_picker(
                        "Symbols",
                        symbols,
                        key_prefix=f"edit_{aid}_{row['id']}",
                    )

                    col1, col2 = st.columns([1, 1])
                    with col1:
                        if updated_symbols != symbols:
                            if st.button("Save Changes", key=f"save_{aid}_{row['id']}"):
                                update_assignment_symbols(aid, row['id'], updated_symbols)
                                st.success(f"Updated symbols to {updated_symbols}")
                                st.rerun()
                    with col2:
                        if st.button("Remove Strategy", key=f"rm_{aid}_{row['id']}", type="secondary"):
                            unassign_strategy_from_account(aid, row['id'])
                            st.rerun()

                    st.divider()

            if st.button("Delete Account", key=f"del_acct_{aid}", type="primary"):
                delete_account(aid)
                st.success(f"Deleted account {aid}")
                st.rerun()
            else:
                st.info("No strategies assigned")
else:
    st.info("No accounts yet. Assign a strategy below to create one.")

st.markdown("---")

# ============ Create Account ============

st.subheader("Create Account")

col1, col2 = st.columns(2)
with col1:
    new_account_id = st.text_input("Account ID", placeholder="e.g. paper_aggressive", key="new_acct_id")
with col2:
    starting_cash = st.number_input("Starting Cash", min_value=1000.0, value=100000.0, step=10000.0, format="%.2f", key="new_acct_cash")

if st.button("Create Account"):
    if not new_account_id.strip():
        st.error("Account ID is required")
    elif new_account_id.strip() in get_all_account_ids():
        st.error("Account already exists")
    else:
        create_account(new_account_id.strip(), starting_cash)
        st.success(f"Created account {new_account_id} with ${starting_cash:,.2f}")
        st.rerun()

st.markdown("---")

# ============ Assign Strategy to Account ============

st.subheader("Assign Strategy to Account")

all_strategies = get_strategies()

if all_strategies.empty:
    st.info("No strategies created yet. Go to the Strategies page to create one.")
    st.stop()

existing_accounts = get_all_account_ids()

if not existing_accounts:
    st.info("Create an account first.")
    st.stop()

col1, col2 = st.columns(2)

with col1:
    account_id = st.selectbox("Account", existing_accounts, key="assign_acct")

with col2:
    strategy_options = {row['name']: row['id'] for _, row in all_strategies.iterrows()}
    selected_name = st.selectbox("Strategy", list(strategy_options.keys()))

symbols = symbol_picker("Symbols", [], key_prefix="assign_new")

if st.button("Assign"):
    if not symbols:
        st.error("At least one symbol is required")
    else:
        strategy_id = strategy_options[selected_name]
        assign_strategy_to_account(account_id, strategy_id, symbols)
        st.success(f"Assigned {selected_name} to {account_id} with {symbols}")
        st.rerun()