# Documentação do projeto

Índice da documentação. Estes documentos são lidos **sob demanda** (não são
importados no `CLAUDE.md`), para não inchar o contexto de cada sessão.

## Arquitetura e padrões

- [rust_clean_arch_guidelines.md](rust_clean_arch_guidelines.md) — **protocolo
  normativo** de Clean Architecture em Rust (camadas, ports, erros, conversões,
  lints, gestão de memória §8.1, checklist de CI §20). Fonte única de verdade
  arquitetural; consulte antes de escrever código Rust.
- [ESTRUTURA_PROJETO.md](ESTRUTURA_PROJETO.md) — estrutura concreta do monorepo:
  stack (front-end Vue 3/TS, back-end Rust com Axum/SQLx), layout de pastas e
  fluxos de trabalho.
- [seguranca-permissoes.md](seguranca-permissoes.md) — subsistema de
  **permissões/segurança** (RBAC/RLS): modelo de dados, autenticação/autorização
  no backend Rust, verificação no frontend, contrato de permissões e checklist.

## Especificações (Spec Kit)

A spec ativa de cada feature vive em `specs/<feature>/` (spec.md → plan.md →
tasks.md). Veja [specs/](../specs/).
