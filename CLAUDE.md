<!-- SPECKIT START -->
For additional context about technologies to be used, project structure,
shell commands, and other important information, read the current plan:
specs/001-satellite-imagery-map/plan.md
(see also research.md, data-model.md, contracts/, quickstart.md in the same dir)
<!-- SPECKIT END -->

# Projeto

Mapa de imagens de satélite. Backend/serviços em **Rust** (edition 2024,
rust-version 1.94), seguindo Clean Architecture com Cargo Workspaces.

Fluxo de trabalho: **Spec Kit**. A especificação ativa vive em
`specs/<feature>/` (spec.md → plan.md → tasks.md). Sempre leia a spec/plan da
feature atual antes de implementar.

# Arquitetura — leitura obrigatória antes de codar Rust

Leia sob demanda (NÃO importados para não inchar o contexto de cada sessão):

- Diretrizes normativas: [docs/rust_clean_arch_guidelines.md](docs/rust_clean_arch_guidelines.md)
- Estrutura de pastas do projeto: [docs/ESTRUTURA_PROJETO.md](docs/ESTRUTURA_PROJETO.md)
- Índice dos docs: [docs/README.md](docs/README.md)

Regras que mais geram erro se ignoradas:

- Dependências apontam para dentro: `presentation`/`infra` → `application` → `domain`.
  `application` **nunca** depende de `infrastructure`.
- Sem `unwrap()`/`expect()` em fluxo de produção; erros de domínio com `thiserror`,
  `anyhow` só em bootstrap/entrypoints.
- `From` só para conversões infalíveis; `TryFrom` para falíveis.
- Composição de dependências só no bootstrap (container via `Arc`).
- Em hot paths, minimizar alocações (ver §8.1): `Cow`, `SmallVec`, `Bytes`, `Arc`,
  iteradores preguiçosos, arenas, `#[repr(transparent)]` — sem otimização prematura.

# Comandos (verificação)

Rode a partir da raiz do workspace antes de considerar o trabalho pronto (espelha
o checklist de CI da §20 das diretrizes):

```bash
cargo check --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
cargo doc --workspace --no-deps
```

Prefira rodar testes específicos durante o desenvolvimento, não a suíte inteira.

# Etiqueta de repositório

- Branches de feature numeradas pelo Spec Kit (ex.: `001-satellite-imagery-map`).
- Commits no padrão Conventional Commits.
- Não commitar/pushar sem pedido explícito; se na branch padrão, criar branch antes.
