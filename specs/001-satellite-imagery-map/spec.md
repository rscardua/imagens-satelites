# Feature Specification: Visualização de Imagens Recentes de Satélite CBERS-4A no Mapa

**Feature Branch**: `001-satellite-imagery-map`

**Created**: 2026-06-23

**Status**: Draft

**Input**: User description: "seguindo as recomendações dos documentos rust_clean_arch_guidelines.md e ESTRUTURA_PROJETO.md crie um sistema em rust com vue que busque imagens recentes de satélite CBERS-4A do inpe ou outra da nasa e exiba no mapa"

## Clarifications

### Session 2026-06-23

- Q: O backend persiste/cacheia as cenas ou só faz proxy ao vivo das fontes? → A: Proxy ao vivo das fontes; cache curto (curta TTL) apenas de metadados de busca. Sem persistência durável de cenas/rasters na v1.
- Q: Quais provedores/protocolos concretos para as fontes? → A: Primária INPE STAC (catálogo CBERS-4A, busca por bbox+intervalo de datas); alternativa NASA GIBS (tiles WMTS de imagery recente).
- Q: Como as imagens são renderizadas no mapa? → A: Footprints (polígonos) clicáveis com metadados + overlay raster da cena priorizada (mais recente); NASA GIBS exibida como camada de tiles.
- Q: Qual política de acesso à visualização na v1? → A: Acesso público, sem autenticação. RBAC do projeto disponível mas não obrigatório no fluxo de busca/exibição.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Visualizar imagens recentes de satélite no mapa (Priority: P1)

Como usuário interessado em sensoriamento remoto, quero abrir o sistema e ver, sobre um mapa interativo, as imagens de satélite mais recentes disponíveis para uma área, para inspecionar visualmente a cobertura terrestre atual sem precisar acessar portais técnicos especializados.

**Why this priority**: É o núcleo do produto. Sem a capacidade de buscar e sobrepor imagens recentes no mapa, nenhuma outra funcionalidade entrega valor. Entrega um MVP demonstrável sozinho.

**Independent Test**: Abrir a aplicação, navegar até uma região com cobertura conhecida e confirmar que pelo menos uma imagem recente é exibida sobreposta ao mapa, com data de aquisição visível.

**Acceptance Scenarios**:

1. **Given** o usuário abre a aplicação com o mapa centralizado em uma área coberta, **When** a aplicação carrega, **Then** as imagens de satélite mais recentes disponíveis para a área visível são sobrepostas ao mapa com indicação da data de aquisição.
2. **Given** o mapa exibe uma área sem cobertura recente, **When** a busca não retorna imagens, **Then** o sistema informa de forma clara que não há imagens recentes para a área e período.
3. **Given** imagens foram carregadas, **When** o usuário consulta os detalhes de uma imagem, **Then** vê data de aquisição, satélite/sensor de origem e fonte (INPE ou NASA).

---

### User Story 2 - Navegar e filtrar por área e período (Priority: P2)

Como usuário, quero mover/dar zoom no mapa e ajustar o intervalo de datas, para encontrar imagens da região e do período de interesse.

**Why this priority**: Amplia a utilidade do MVP permitindo exploração dirigida, mas depende da capacidade base de exibir imagens (US1).

**Independent Test**: Mover o mapa para uma nova região e/ou alterar o intervalo de datas e confirmar que a lista de imagens sobrepostas é atualizada para refletir a nova área/período.

**Acceptance Scenarios**:

1. **Given** o usuário arrasta o mapa para uma nova região, **When** a navegação termina, **Then** a busca é refeita e as imagens da nova área são exibidas.
2. **Given** o usuário define um intervalo de datas, **When** aplica o filtro, **Then** apenas imagens adquiridas dentro do intervalo são consideradas.
3. **Given** múltiplas imagens cobrem a mesma área, **When** exibidas, **Then** a mais recente tem precedência visual e o usuário pode percorrer as demais por data.

---

### User Story 3 - Comparar fontes INPE e NASA (Priority: P3)

Como usuário, quero escolher ou alternar entre a fonte INPE (CBERS-4A) e uma fonte NASA, para comparar disponibilidade e atualidade das imagens da mesma área.

**Why this priority**: Agrega valor de comparação e cobertura redundante, mas o produto é viável com uma única fonte ativa.

**Independent Test**: Selecionar a fonte INPE, observar as imagens; alternar para a fonte NASA e confirmar que o conjunto exibido muda conforme a fonte selecionada.

**Acceptance Scenarios**:

1. **Given** o seletor de fonte está visível, **When** o usuário escolhe INPE (CBERS-4A), **Then** apenas imagens dessa fonte são exibidas.
2. **Given** a fonte INPE está indisponível, **When** a busca falha, **Then** o sistema sinaliza a indisponibilidade e oferece a fonte NASA como alternativa.

---

### Edge Cases

- **Sem cobertura**: área/período sem imagens — exibir mensagem clara, sem estado de erro confuso.
- **Cobertura por nuvens**: imagem recente fortemente encoberta — sinalizar percentual de nuvens quando disponível para o usuário julgar a utilidade.
- **Fonte externa indisponível ou lenta**: timeout ou erro do provedor — degradar graciosamente, informar o usuário e permitir nova tentativa.
- **Área muito ampla**: zoom muito afastado pode retornar volume excessivo de imagens — limitar resultados e orientar o usuário a aproximar.
- **Imagem parcial sobre a área visível**: sobrepor apenas a porção relevante sem distorcer o mapa.
- **Limites de uso do provedor**: respeitar limites de requisição da fonte externa sem quebrar a experiência.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST buscar, em uma ou mais fontes externas de imagens de satélite, as imagens disponíveis para uma área geográfica e um período informados.
- **FR-002**: O sistema MUST suportar como fonte primária as imagens do satélite CBERS-4A do INPE via catálogo STAC (busca por extensão geográfica e intervalo de datas) e MUST suportar como alternativa a NASA GIBS (tiles WMTS de imagery recente).
- **FR-003**: O sistema MUST exibir as imagens encontradas sobrepostas a um mapa interativo, posicionadas geograficamente de forma correta, representando cada cena por seu footprint (polígono) clicável com metadados e sobrepondo o raster da cena priorizada; fontes em tiles (NASA GIBS) MUST ser exibidas como camada de tiles.
- **FR-004**: O sistema MUST priorizar, por padrão, as imagens mais recentes para a área visível.
- **FR-005**: Usuários MUST conseguir navegar (mover e dar zoom) no mapa e ter a busca de imagens atualizada conforme a área visível.
- **FR-006**: Usuários MUST conseguir definir um intervalo de datas para filtrar as imagens exibidas.
- **FR-007**: O sistema MUST exibir, para cada imagem, metadados essenciais: data de aquisição, satélite/sensor de origem e fonte (INPE ou NASA).
- **FR-008**: O sistema MUST informar claramente quando não houver imagens para a área e período consultados.
- **FR-009**: O sistema MUST tratar indisponibilidade, lentidão ou erro das fontes externas de forma graciosa, comunicando o estado ao usuário e permitindo nova tentativa.
- **FR-010**: O sistema MUST sinalizar a cobertura de nuvens da imagem quando essa informação estiver disponível na fonte.
- **FR-011**: O sistema MUST limitar o volume de resultados retornados para áreas amplas e orientar o usuário a refinar a busca.
- **FR-012**: Usuários MUST conseguir selecionar a fonte de imagens (INPE ou NASA) quando ambas estiverem disponíveis para a área.
- **FR-013**: O sistema MUST respeitar os limites de uso e políticas das fontes externas de imagens.
- **FR-014**: O sistema MUST manter a separação entre regra de negócio (seleção/priorização de imagens) e a integração com cada fonte externa, de modo que novas fontes possam ser adicionadas sem alterar o núcleo de negócio.
- **FR-015**: O sistema MUST consultar as fontes externas ao vivo a cada busca (proxy), sem manter persistência durável de cenas ou rasters na v1; é permitido apenas cache de curta duração dos metadados de busca para reduzir latência e chamadas redundantes às fontes.

### Key Entities *(include if feature involves data)*

- **Imagem de Satélite (Cena)**: representa uma captura disponível. Atributos: identificador, data/hora de aquisição, área de cobertura (extensão geográfica), satélite/sensor, fonte (INPE/NASA), cobertura de nuvens (quando disponível), referência para a visualização da imagem.
- **Fonte de Imagens (Provedor)**: representa um provedor externo (INPE/CBERS-4A, NASA). Atributos: identificação, disponibilidade, capacidades suportadas (filtros por área/data, metadados oferecidos).
- **Consulta de Área**: parâmetros de uma busca do usuário. Atributos: extensão geográfica visível, intervalo de datas, fonte selecionada, ordenação por recência.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A partir da abertura da aplicação em uma área com cobertura conhecida, o usuário vê pelo menos uma imagem recente sobreposta ao mapa em até 10 segundos.
- **SC-002**: Para uma área com cobertura disponível, a imagem exibida por padrão é a mais recente entre as encontradas em pelo menos 95% das consultas.
- **SC-003**: Quando não há imagens para a área/período, o usuário recebe uma mensagem clara em 100% dos casos, sem estado de erro ambíguo.
- **SC-004**: Quando uma fonte externa está indisponível, o usuário é informado e consegue tentar novamente ou alternar de fonte sem recarregar manualmente a aplicação inteira.
- **SC-005**: Um novo usuário consegue localizar imagens de uma região de interesse e identificar a data de aquisição sem instruções externas, em até 2 minutos.
- **SC-006**: A informação de data de aquisição e fonte está visível para 100% das imagens exibidas.

## Assumptions

- A fonte primária é o satélite CBERS-4A do INPE; a fonte alternativa NASA cobre lacunas de disponibilidade/atualidade (assume-se uma fonte pública de imagens recentes acessível por área e data).
- Verificado (2026-06-23) que a STAC pública do INPE (Brazil Data Cube) disponibiliza, para CBERS-4A, o produto WFI em composto de 8 dias (`CBERS-WFI-8D-1`); cenas individuais de alta resolução (WPM/MUX) ficam no catálogo DGI, fora desta STAC. Logo, na v1 as "imagens recentes" do CBERS-4A são os compostos WFI de 8 dias; WPM/MUX por cena ficam fora de escopo, com a arquitetura permitindo adicionar essa fonte depois sem mudar o núcleo.
- "Recente" significa, por padrão, as imagens mais novas disponíveis na fonte, com filtro de datas configurável pelo usuário; assume-se janela padrão dos últimos 30 dias quando o usuário não especifica.
- O acesso às fontes externas é feito por integração programática pública; eventuais credenciais/chaves de acesso são tratadas como configuração operacional, não como funcionalidade do usuário.
- O escopo da v1 é visualização e exploração de imagens no mapa; download em lote, processamento avançado de imagens e análise espectral estão fora de escopo.
- O acesso à visualização é público na v1 (sem autenticação). O RBAC do backend existente permanece disponível no monorepo, mas não é obrigatório no fluxo de busca/exibição; recursos futuros (ex.: export, salvar área) podem exigir autenticação.
- O frontend opera em timezone local e exibe datas de aquisição de forma compatível com essa expectativa.
- Suporte a desktop (navegador moderno) é o alvo da v1; otimização específica para mobile é desejável mas não obrigatória.
