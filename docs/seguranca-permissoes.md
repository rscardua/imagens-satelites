# Sistema de Permissões e Segurança

> Extraído de [ESTRUTURA_PROJETO.md](ESTRUTURA_PROJETO.md) (§5). Índice geral em
> [README.md](README.md). A numeração `5.x` é preservada para manter referências.

O controle de acesso é baseado em **RBAC (Role-Based Access Control)** granular, operando tanto no nível da interface (Frontend) quanto na API (Backend). A arquitetura implementa dois níveis de proteção: **verificação de acesso** (o usuário pode executar a operação?) e **filtros de dados** (quais registros ele pode visualizar/modificar?).

## 5.1 Modelo de Dados (Database)

A estrutura relacional que sustenta as permissões envolve as seguintes tabelas principais:

- **`usuario`**: O ator que realiza a ação.
- **`perfil`**: Agrupador de permissões (ex: "Administrador", "Técnico Ambiental", "Visualizador").
- **`usuario_perfil`**: Tabela de ligação (N:N) entre usuários e perfis. Um usuário pode ter múltiplos perfis.
- **`recurso`**: Representa um endpoint ou conjunto de endpoints da API (ex: `/api/usuario`, `/api/licenca/:%`). Suporta wildcards (`:%)` para matching dinâmico de UUIDs e parâmetros.
- **`operacao`**: Ação específica do sistema (ex: "Criar Usuário", "Visualizar Licença", "Excluir Relatório"). Possui flags `publico` e `qualquer_autenticado`.
- **`operacao_recurso`**: Liga operações a recursos específicos (ex: POST em `/api/usuario` = operação "Criar Usuário").
- **`autorizacao`**: Define que um `perfil` tem permissão para executar uma `operacao`. Possui flag `ignorar_filtro` para bypass de filtros de dados.

**Diagrama de Relacionamentos:**
```
usuario (1) ←──→ (N) usuario_perfil (N) ←──→ (1) perfil
                                                    ↓
                                                    | (1)
                                                    ↓
                                            autorizacao (N) ←──→ (1) operacao
                                                                        ↓
                                                                        | (1)
                                                                        ↓
                                                                operacao_recurso (N) ←──→ (1) recurso
```

## 5.2 Lógica no Backend Rust (`apps/server-rust`)
O backend Rust é a implementação principal do modelo RBAC/RLS, com a lógica distribuída em componentes menores e explicitamente compostos.

### 5.2.1 Fluxo de autorização no Rust
- **Resolução de token**:
  - `middleware/auth.rs` contém dois middlewares:
    - `resolve_access_token`: token opcional; se existir e for válido, injeta `AuthenticatedUser`.
    - `require_access_token`: token obrigatório; falha com 401 quando ausente ou inválido.
- **Verificação de permissão e filtro**:
  - `middleware/rbac.rs` recebe `method + path + current_user`.
  - Consulta a relação `recurso -> operacao_recurso -> operacao`.
  - Se a rota for pública, libera.
  - Se exigir autenticação simples, injeta `AuditUserId` e segue.
  - Se exigir autorização, consulta permissões do usuário e resolve `RlsFilter`.
- **Aplicação do filtro**:
  - Em handlers de busca/exportação, o filtro de RLS é combinado com o filtro da request por `merge_search_params_with_rls`.
  - Isso preserva o contrato esperado pelo cliente sem mutações implícitas e difíceis de rastrear no payload original.
- **Auditoria**:
  - Em operações de escrita, os handlers recebem `AuditUserId` via extension e populam `operacao_usuario_id`.

### 5.2.2 Composição Modular de Segurança
A segurança no backend Rust é separada em componentes claros e independentes:
- **middlewares de autenticação**: Resolvem e validam tokens JWT.
- **middleware RBAC**: Verifica permissões e resolve filtros de dados.
- **helpers de merge de filtro**: Combinam filtros RLS com parâmetros de busca.
- **handlers explícitos**: Aplicam auditoria (`operacao_usuario_id`) antes da persistência.

Esta decomposição torna o sistema mais previsível para manutenção, testes e debugging.

### 5.2.3 Sincronização de Rotas e Catálogo de Recursos
- O backend Rust usa a tabela `recurso` como referência de autorização, mantendo compatibilidade com o modelo RBAC/RLS.
- O catálogo de rotas nasce no código dos módulos HTTP via macro CRUD ou declaração manual.
- As rotas são sincronizadas automaticamente no startup via `upsert`, eliminando a necessidade de listas estáticas paralelas.

## 5.3 Tabelas SQL e Contrato de Permissões

O modelo RBAC/RLS gira em torno de cinco tabelas principais e um contrato simples consumido pelo frontend.

- **`operacao`**: catálogo de capacidades do sistema. O campo `nome_interno` é a chave funcional consumida pelo frontend como `nomeInterno`.
- **`recurso`**: descreve rota HTTP, método e filtro RLS opcional (`filtro_para_aplicar`).
- **`operacao_recurso`**: liga uma ou mais operações a um recurso HTTP concreto.
- **`perfil`**: agrupador de permissões, com `admin = true` como bypass administrativo global.
- **`autorizacao`**: relaciona perfil e operação, incluindo a flag `ignorar_filtro`.
- **`usuario_perfil`**: vínculo N:N entre usuário e perfis.

Exemplos de chaves compatíveis com o projeto atual:

```sql
INSERT INTO operacao (nome_interno, nome, publico, qualquer_autenticado) VALUES
  ('usuario-add', 'Adicionar usuário', false, false),
  ('usuario-edit', 'Editar usuário', false, false),
  ('admin/datasource-add', 'Adicionar fonte de dados', false, false),
  ('auth.permission.any', 'Acesso autenticado genérico', false, true);
```

Exemplos de recursos sincronizados pelo backend:

```sql
INSERT INTO recurso (nome, url, metodo, filtro_para_aplicar) VALUES
  ('Criar usuário', '/api/usuario', 'POST', NULL),
  ('Pesquisar usuário', '/api/usuario/search', 'POST', NULL),
  ('Minhas permissões', '/api/usuario/me/permission', 'GET', NULL);
```

O frontend não persiste uma ACL própria: ele consome `GET /api/usuario/me/permission`, cujo payload atual contém `id`, `nome` e `nomeInterno` em camelCase.

## 5.4 Lógica no Frontend (`apps/client`)

O frontend utiliza uma estratégia **proativa** para melhorar a experiência do usuário (UX), mas **nunca** substitui a validação de segurança do backend.

### 5.4.1 Store de sessão e permissões

A store `general` concentra o estado de sessão da SPA:

- dados do usuário autenticado;
- `accessToken` e `refreshToken`;
- tokens auxiliares para fluxos OAuth (`keycloakAccessToken`, `keycloakRefreshToken`);
- lista de permissões (`userData.permissions`);
- estratégia de autenticação disponível em `authStrategy`.

Fluxo atual:

```typescript
const user = await new SignInUseCase(new UsuarioApiRepository()).execute(signInData)
store.setUserSession(user)

const setPermissionsUseCase = new SetPermissionUseCase(store, new UsuarioApiRepository())
await setPermissionsUseCase.execute()
```

O método central de verificação continua sendo:

```typescript
checkPermission(permissions: string[]) {
  return permissions.every((item) =>
    this.userData?.permissions?.some(
      (p) => p.nomeInterno === item.replace(/^\//, '')
    )
  )
}
```

Isso permite dois padrões reais de uso no projeto:

- chaves semânticas como `usuario-add`, `perfil-edit` e `admin/mapa-base-add`;
- checks derivados de `r.path` em menus, como `/usuario-list` e `/perfil-list`, normalizados pela remoção da barra inicial.

### 5.4.2 Verificação visual de permissões

Telas CRUD usam permissões explícitas:

```vue
<button
  v-if="store.checkPermission(['usuario-add'])"
  class="nt-btn"
  @click="onAdd($route, $router)"
>
  Novo Usuário
</button>
```

Menus e sidebars combinam metadados de rota com permissões por path:

```typescript
const hasPermission = r.meta?.isPublic || store.checkPermission([r.path])
const shouldShow = r.meta?.isPublic || (!r.meta?.menuHidden && store.checkPermission([r.path]))
```

### 5.4.3 HttpClient, refresh e legado

O `HttpClient` em `@core/@shared/infra/http/http-client.ts` centraliza:

- envio do `Authorization` com `accessToken`;
- refresh automático via `GET /usuario/me/refresh` quando recebe `401`;
- atualização de `accessToken` e `refreshToken` na store;
- formatação de erros antes de propagar para a UI.

Há repositórios legados que ainda enviam `headers: localStorage.getItem('token')` em chamadas específicas. Isso existe no código atual, mas o padrão arquitetural vigente continua sendo o uso do `HttpClient` e da store global.

### 5.4.4 Estratégias de autenticação

O frontend consulta `GET /api/usuario/me/auth-config` para descobrir as estratégias habilitadas no backend. No estado atual, o projeto suporta:

- `local`;
- `keycloak`;
- `google`.

`router.ts` já trata `OAuth Callback` como rota pública, e `AuthConfigService` usa `sessionStorage` para guardar `state` e `nonce` dos fluxos OAuth.

### 5.4.5 Segurança real vs validação visual

| Aspecto | Frontend (Vue 3) | Backend (Rust) |
| ------- | ---------------- | -------------- |
| Propósito | Melhorar UX e evitar ações inúteis | Garantir segurança efetiva |
| Fonte da decisão | `store.userData.permissions` | Banco + middlewares + contexto autenticado |
| Adulterável | Sim, via browser/devtools | Não, a validação é server-side |
| Filtro de dados | Não aplica RLS | Aplica `filtro_para_aplicar` quando necessário |

Mesmo que um usuário altere o `localStorage`, o backend continua consultando permissões reais em `operacao`, `autorizacao`, `perfil` e `usuario_perfil`, retornando `403` quando a permissão não existir de fato.

## 5.5 Exemplos práticos do fluxo atual

### 5.5.1 Tela CRUD de usuário

- `pages/Usuario/List.vue` usa `SearchUsuarioUseCase`, `DeleteUsuarioUseCase` e `ExportXlsxUsuarioUseCase`.
- O botão de criação depende de `store.checkPermission(['usuario-add'])`.
- A requisição segue com `Authorization` injetado pelo `HttpClient`.
- O backend valida o recurso HTTP correspondente e persiste com auditoria via `operacao_usuario_id`.

### 5.5.2 Navegação e menus

- `routes.ts` concentra ícones, descrições, visibilidade e hierarquia (`subMenuName`).
- `router.ts` aplica autenticação e redirecionamento.
- Componentes de navegação usam `r.path` e `checkPermission()` para determinar se uma rota deve aparecer no menu.

### 5.5.3 Logout e fluxos OAuth

No logout, a store identifica `tipoLogin` e envia `x-oauth-refresh-token` quando a sessão atual veio de Keycloak ou Google:

```typescript
if (this.userData?.tipoLogin === 'keycloak' || this.userData?.tipoLogin === 'google') {
  oauthRefreshToken = this.userData?.keycloakRefreshToken
}

await new SignOutUseCase(new UsuarioApiRepository()).execute(oauthRefreshToken)
```

## 5.6 Ciclo resumido da requisição

```text
Frontend Vue 3
  -> autentica usuário e carrega permissões em /api/usuario/me/permission
  -> decide visibilidade de telas, botões e menus com checkPermission()
  -> envia requisição com Authorization via HttpClient

Backend Rust
  -> valida token em auth.rs
  -> resolve recurso + operação + permissões em rbac.rs
  -> aplica filtro RLS quando necessário
  -> executa handler/use case/repositório
  -> persiste auditoria e retorna resposta HTTP

Frontend Vue 3
  -> trata sucesso/erro
  -> tenta refresh em 401
  -> atualiza UI
```

## 5.7 Checklist de segurança para desenvolvedores

Ao implementar novos recursos, sempre verifique:

**Backend:**
- [ ] Recurso cadastrado/sincronizado com URL e método corretos
- [ ] `nome_interno` da operação é estável e coerente com o uso no frontend
- [ ] `operacao_recurso` cobre todos os recursos HTTP necessários
- [ ] Perfis e autorizações foram configurados corretamente
- [ ] `ignorar_filtro` foi avaliado explicitamente
- [ ] Recursos com RLS possuem `filtro_para_aplicar` válido
- [ ] Handlers não duplicam validação de permissão já feita no middleware
- [ ] Entidades persistidas preservam `operacao_usuario_id`, `created_at` e `updated_at`

**Frontend:**
- [ ] Componentes usam `store.checkPermission([...])` para esconder ações inacessíveis
- [ ] As chaves usadas em `checkPermission()` correspondem ao `nomeInterno` retornado por `/api/usuario/me/permission`
- [ ] Guards e rotas públicas estão alinhados entre `router.ts` e `routes.ts`
- [ ] Fluxos OAuth consideram `auth-config`, `OAuth Callback` e logout com `x-oauth-refresh-token`
- [ ] Erros `401` e `403` têm tratamento visível e previsível na UI

**Testes:**
- [ ] Cobrir usuário sem permissão (`403`)
- [ ] Cobrir usuário com múltiplos perfis
- [ ] Cobrir aplicação de filtros RLS quando aplicável
- [ ] Verificar persistência de `operacao_usuario_id`
- [ ] Verificar que o frontend não exibe ações sem permissão
