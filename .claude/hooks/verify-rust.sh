#!/usr/bin/env bash
# Stop-hook gate: roda o checklist de verificação Rust (espelha a §20 das
# diretrizes). No-op enquanto não existir o workspace Cargo, para não atrapalhar
# a fase de especificação. Exit 2 bloqueia o término do turno e devolve o erro
# ao Claude; exit 0 permite parar.
set -uo pipefail

cd "${CLAUDE_PROJECT_DIR:-.}" 2>/dev/null || exit 0

# Ainda sem código Rust → nada a verificar.
[ -f Cargo.toml ] || exit 0
command -v cargo >/dev/null 2>&1 || exit 0

run() {
  local out
  if ! out=$("$@" 2>&1); then
    echo "Verificação falhou: $*" >&2
    echo "$out" | tail -n 40 >&2
    exit 2
  fi
}

run cargo fmt --all -- --check
run cargo clippy --workspace --all-targets --all-features
run cargo test --workspace
exit 0
