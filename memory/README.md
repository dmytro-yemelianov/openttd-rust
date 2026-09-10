# Project Memory (`memory/`)

This directory provides compact, sharded knowledge cards for coding agents working on `openttd-rust`.
It is specifically designed for token-constrained workers (e.g. 16K context window limits) so agents never have to ingest entire specification files or full Rust modules just to find types, architectures, or task requirements.

## Navigation Structure

| Shard | Purpose | Typical Token Size |
|---|---|---|
| [`memory/state.json`](state.json) | Machine-readable source of truth for repair pipeline status & active task | ~150 tokens |
| [`memory/architecture.md`](architecture.md) | Crate boundaries, dependency graph, layer responsibilities | ~400 tokens |
| [`memory/data_model.md`](data_model.md) | Cheat sheet of core IDs, units, entities, enums, and signatures | ~500 tokens |
| [`memory/safeguards.md`](safeguards.md) | Upstream translation rules: determinism, explicit bounds, caches | ~350 tokens |
| [`memory/tasks/`](tasks/) | Individual ~30-line bounded task cards for each repair step | ~250 tokens each |

## Usage Rules for Agents

1. **Never load all shards at once**:
   - Only load [`memory/state.json`](state.json) and the specific active task card in [`memory/tasks/`](tasks/).
2. **Consult data model before reading `.rs` files**:
   - Check [`memory/data_model.md`](data_model.md) for existing struct/enum fields before reading crate source files.
3. **Use `tools/ctx.py` CLI**:
   - Run `python3 tools/ctx.py status` to see where the project currently stands.
   - Run `python3 tools/ctx.py check` to check compilation without compiler warning spam.
   - Run `python3 tools/ctx.py test` to test with compact output.
   - Run `python3 tools/ctx.py slice <path> <symbol_or_lines>` to read small, targeted code chunks.
4. **Use MCP Graph Discovery for Code**:
   - Use `codebase-memory-mcp` tools (`search_graph`, `trace_path`, `get_code_snippet`) for precise symbol-level lookups instead of whole-file grepping or bulk file viewing.
