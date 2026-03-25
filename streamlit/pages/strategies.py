import streamlit as st
import json
import sys
sys.path.append('..')
from utils.db import get_strategies, insert_strategy, update_strategy, delete_strategy
from strategies import STRATEGY_REGISTRY

st.set_page_config(page_title="Strategies", layout="wide")
st.title("Strategies")

if st.button("Refresh"):
    st.rerun()

# ============ Existing Strategies ============

st.subheader("Existing Strategies")

strategies_df = get_strategies()

if not strategies_df.empty:
    for _, row in strategies_df.iterrows():
        params = json.loads(row['params_json']) if row['params_json'] else {}
        param_type = params.get("type", row['strategy_type'])
        definition = STRATEGY_REGISTRY.get(param_type)

        with st.expander(f"{row['name']} — {row['strategy_type']}"):
            st.caption(f"ID: {row['id']}  |  Created: {row['created_at']}")

            new_name = st.text_input("Name", value=row['name'], key=f"name_{row['id']}")

            if definition:
                st.write(f"**{definition.name()} Parameters**")
                descs = definition.param_descriptions()

                edited_params = {}
                col1, col2 = st.columns(2)

                param_keys = [k for k in params if k != "type"]
                half = (len(param_keys) + 1) // 2

                for i, key in enumerate(param_keys):
                    col = col1 if i < half else col2
                    value = params[key]
                    label = key.replace('_', ' ').title()

                    with col:
                        if isinstance(value, float):
                            edited_params[key] = st.number_input(
                                label, value=value, step=0.005, format="%.3f",
                                key=f"param_{row['id']}_{key}"
                            )
                        elif isinstance(value, int):
                            edited_params[key] = st.number_input(
                                label, value=value, min_value=1,
                                key=f"param_{row['id']}_{key}"
                            )
                        elif key.endswith("_lookback") or key.endswith("_timeframe"):
                            options = ["1Min", "5Min", "15Min", "30Min", "1Hour", "1Day"]
                            idx = options.index(value) if value in options else 0
                            edited_params[key] = st.selectbox(
                                label, options, index=idx,
                                key=f"param_{row['id']}_{key}"
                            )
                        else:
                            edited_params[key] = st.text_input(
                                label, value=str(value),
                                key=f"param_{row['id']}_{key}"
                            )

                        desc = descs.get(key, "")
                        if desc:
                            st.caption(desc)

                # Check for changes
                name_changed = new_name != row['name']
                params_changed = edited_params != {k: v for k, v in params.items() if k != "type"}

                col1, col2 = st.columns([1, 1])
                with col1:
                    if name_changed or params_changed:
                        if st.button("Save Changes", key=f"save_{row['id']}"):
                            new_params = {"type": param_type, **edited_params}
                            update_strategy(
                                row['id'],
                                new_name,
                                row['strategy_type'],
                                new_params,
                            )
                            st.success("Saved")
                            st.rerun()
                    else:
                        st.button("Save Changes", key=f"save_{row['id']}", disabled=True)

                with col2:
                    if st.button("Delete", key=f"del_{row['id']}", type="primary"):
                        delete_strategy(row['id'])
                        st.success(f"Deleted {row['name']}")
                        st.rerun()
            else:
                st.json(params)
                if st.button("Delete", key=f"del_{row['id']}", type="primary"):
                    delete_strategy(row['id'])
                    st.rerun()
else:
    st.info("No strategies created yet.")

st.markdown("---")

# ============ Create Strategy ============

st.subheader("Create Strategy")

strategy_types = list(STRATEGY_REGISTRY.keys())
selected_type = st.selectbox("Strategy Type", strategy_types)
definition = STRATEGY_REGISTRY[selected_type]

name = st.text_input("Strategy Name", placeholder=f"e.g. {definition.name().lower()}_v1")

st.write(f"**{definition.name()} Parameters**")
params = definition.render_params(st)

if st.button("Create Strategy"):
    if not name.strip():
        st.error("Strategy name is required")
    else:
        params_json = definition.to_params_json(params)
        strategy_id = insert_strategy(name.strip(), selected_type, params_json)
        st.success(f"Created strategy '{name}' (ID: {strategy_id})")
        st.rerun()
