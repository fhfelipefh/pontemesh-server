# Origin API

Este documento descreve a API conceitual do **Origin** no Ponte Mesh.

O Origin é a autoridade central da arquitetura. Ele concentra o plano de controle, mantém o catálogo de objetos, emite autorizações, gera ou disponibiliza manifestos, controla revogação, aplica políticas e atua como fonte direta ou fonte final de garantia no plano de dados.

A API do Origin deve ser organizada em grupos de responsabilidade, preservando a separação entre operações compatíveis com o modelo S3-like e operações específicas do Ponte Mesh.

## Grupos de API

A API do Origin deve ser dividida conceitualmente nos seguintes grupos:

* **S3-like**: buckets, objetos, metadados, `GET`, `PUT`, `HEAD`, `DELETE` e recuperação por intervalo de bytes.
* **Controle Ponte Mesh**: manifestos, pacotes de acesso, disponibilidade, políticas, fontes autorizadas, fallback e revogação.
* **Replica/Edge**: registro, autenticação, autorização, plano de sincronização, anúncio de disponibilidade, revogação e métricas.
* **Observabilidade**: métricas, auditoria, saúde do serviço e eventos operacionais.
* **Administração**: operações administrativas futuras para painel, políticas, configuração, governança e integrações como MCP.

## Diretrizes gerais

A API do Origin deve seguir as seguintes diretrizes:

* toda obtenção controlada de conteúdo deve começar com autorização do Origin;
* pacotes de acesso devem ser emitidos apenas pelo Origin;
* manifestos devem ser emitidos, assinados ou validados pelo Origin;
* operações administrativas devem exigir autenticação, autorização e auditoria;
* Replica/Edge opera sob autorização do Origin;
* fragmentos recebidos de peers devem ser validados;
* funcionalidades específicas do Ponte Mesh ficam em APIs próprias;
* operações base de bucket e objeto devem permanecer preferencialmente na API S3-like;
* políticas, manifestos, fontes autorizadas, fallback e métricas devem ficar na API Ponte Mesh.

## API S3-like

A API S3-like deve oferecer uma interface familiar para operações fundamentais de buckets e objetos.

Ela deve permitir que aplicações existentes, dentro do subconjunto suportado, consigam apontar para o endpoint do Origin com mudanças mínimas de integração.

A implementação atual separa o painel web/admin e a API S3-compatible por porta:
o painel fica em `http://localhost:8080`, e o endpoint S3-compatible fica em
`http://localhost:9000`. Na porta S3, as operações ficam na raiz do endpoint.

### Responsabilidades

A API S3-like deve cobrir:

* criação de buckets;
* listagem de buckets;
* envio de objetos;
* listagem de objetos;
* consulta de metadados por `HEAD`;
* recuperação de objeto por `GET`;
* recuperação parcial por `Range`;
* remoção lógica de objeto;
* geração de URL temporária ou mecanismo equivalente.

### Operações conceituais

#### Criar bucket

```http
PUT /{bucket}
```

Cria um bucket lógico no Origin.

Deve exigir autenticação e autorização administrativa ou de aplicação com escopo adequado.

#### Listar buckets

```http
GET /
```

Lista buckets visíveis para a entidade autenticada.

A resposta deve respeitar escopos e políticas de visibilidade.

#### Enviar objeto

```http
PUT /{bucket}/{objectKey}
```

Envia um objeto ao Origin.

O Origin deve registrar o objeto no catálogo, armazenar o conteúdo primário, gerar metadados e preparar as informações necessárias para manifesto, fragmentação e políticas futuras.

#### Listar objetos

```http
GET /{bucket}
```

Lista objetos de um bucket.

A listagem deve respeitar autenticação, autorização, filtros, paginação e políticas de visibilidade.

#### Consultar metadados

```http
HEAD /{bucket}/{objectKey}
```

Consulta metadados de um objeto sem transferir seu conteúdo.

Pode retornar informações como tamanho, tipo de conteúdo, versão, estado de disponibilidade, data de modificação e metadados relevantes.

#### Recuperar objeto

```http
GET /{bucket}/{objectKey}
```

Recupera um objeto.

Externamente, a chamada deve se comportar como uma operação S3-like. Internamente, o Origin pode aplicar autorização, manifesto, políticas, registro de métricas e controle de disponibilidade.

#### Recuperar intervalo de bytes

```http
GET /{bucket}/{objectKey}
Range: bytes=start-end
```

Recupera parte do objeto.

Essa operação é essencial para retomada parcial, fallback por fragmento e obtenção eficiente de partes específicas do objeto.

#### Remover logicamente objeto

```http
DELETE /{bucket}/{objectKey}
```

Marca o objeto como removido, revogado ou indisponível conforme a política aplicável.

A remoção lógica deve impedir novas autorizações de obtenção, mas não deve prometer apagamento físico imediato de cópias transitórias já distribuídas fora do Origin.

#### Gerar URL temporária ou equivalente

A API deve permitir um mecanismo seguro de acesso temporário.

Esse mecanismo pode ser uma URL assinada, ticket temporário, token opaco ou outro contrato equivalente, desde que preserve escopo, expiração e revogabilidade.

## API de Controle Ponte Mesh

A API de Controle Ponte Mesh deve expor recursos específicos da arquitetura híbrida que não cabem naturalmente no modelo S3-like.

Essa API é usada por SDKs, ferramentas administrativas, dashboard futuro e integrações operacionais.

### Responsabilidades

A API de Controle Ponte Mesh deve permitir:

* emitir pacote de acesso;
* consultar manifesto autorizado;
* consultar disponibilidade de objetos e fragmentos;
* consultar fontes autorizadas;
* consultar políticas aplicáveis;
* configurar políticas de distribuição híbrida;
* configurar estratégias de fallback;
* configurar priorização de fragmentos;
* revogar objetos, usuários, aplicações, pacotes de acesso e fontes;
* registrar métricas reportadas pelo SDK;
* fornecer contratos estáveis para SDKs.

### Obter pacote de acesso

```http
POST /pontemesh/access-packages
```

Emite um pacote de acesso para uma obtenção específica.

O pacote de acesso pode conter:

* identificação do objeto;
* manifesto autorizado;
* credencial ou ticket temporário;
* prazo de expiração;
* fontes autorizadas;
* política de seleção de fragmentos;
* política de seleção de fontes;
* endpoints de fallback;
* restrições aplicáveis.

O Origin deve negar a emissão quando não houver autenticação, autorização ou política válida.

O contrato inicial recebe:

```json
{
  "bucket": "exemplo",
  "key": "objeto.bin",
  "ttlSeconds": 300
}
```

E retorna um pacote temporário com token opaco, fonte autorizada `ORIGIN`,
fallback pelo endpoint S3-compatible `/{bucket}/{objectKey}` e manifesto
emitido pelo Origin.

O `ttlSeconds` solicitado deve respeitar o máximo definido na política
persistida do bucket. Quando omitido, o TTL padrão também vem dessa política.

### Consultar manifesto autorizado

```http
GET /pontemesh/objects/{bucket}/manifest/{objectKey}
```

Retorna o manifesto de um objeto quando a entidade solicitante possui autorização válida.

O manifesto deve conter informações suficientes para o SDK obter, validar e reconstruir logicamente o objeto.

O tamanho de fragmento do manifesto vem da política persistida do bucket.
Objetos em estado diferente de `AVAILABLE`, incluindo `REVOKED`, não geram
manifesto nem pacote de acesso.

### Política de bucket

```http
GET /api/admin/buckets/{bucket}/policy
PUT /api/admin/buckets/{bucket}/policy
```

Contrato inicial:

```json
{
  "accessPackageTtlSeconds": 900,
  "fragmentSizeBytes": 4194304,
  "allowReplicaEdge": false,
  "allowPeerSharing": false
}
```

Essa política é persistida no catálogo PostgreSQL. O Origin usa
`accessPackageTtlSeconds` na emissão de pacotes e `fragmentSizeBytes` na geração
de manifestos.

### Consultar disponibilidade

```http
GET /pontemesh/objects/{objectId}/availability
```

Consulta o estado de disponibilidade de um objeto ou de seus fragmentos.

Na implementação atual, o contrato SDK-facing usa bucket e chave para manter o
mesmo padrão das rotas de manifesto e fontes:

```http
GET /pontemesh/objects/{bucket}/availability/{objectKey}
```

A chamada exige credencial de aplicação com escopo
`pontemesh:availability:read` e retorna a disponibilidade do Origin, de
Replica/Edge e de peers autorizados por fragmento.

Estados conceituais possíveis:

* `AVAILABLE`;
* `EXPIRED`;
* `REVOKED`;
* `UNAVAILABLE`;
* `BLOCKED`.

### Consultar fontes autorizadas

```http
GET /pontemesh/objects/{objectId}/sources
```

Retorna fontes autorizadas para determinada obtenção, respeitando pacote de acesso, política, expiração e escopo.

Tipos de fonte possíveis:

* `ORIGIN`;
* `REPLICA_EDGE`;
* `PEER`.

O Origin deve ser sempre a fonte final de garantia. Replica/Edge e peers são fontes auxiliares condicionadas à política aplicável.

### Consultar políticas aplicáveis

```http
GET /pontemesh/objects/{objectId}/policies
```

Retorna políticas aplicáveis ao objeto, bucket, aplicação ou contexto de obtenção.

Na implementação atual, o contrato SDK-facing usa bucket e chave:

```http
GET /pontemesh/objects/{bucket}/policies/{objectKey}
```

A chamada exige credencial de aplicação com escopo `pontemesh:policies:read` e
retorna a política efetiva necessária para seleção de fontes, fragmentação,
fallback e revalidação.

As políticas podem incluir:

* permissão ou bloqueio de P2P;
* permissão ou bloqueio de Replica/Edge;
* prioridade entre fontes;
* limites de fallback;
* expiração de pacote de acesso;
* estratégia de fragmentação;
* estratégia de priorização de fragmentos;
* regras de revalidação;
* limites de vazão, latência e falhas.

### Configurar política de distribuição

```http
PUT /pontemesh/policies/{policyId}
```

Permite criar ou atualizar políticas específicas do Ponte Mesh.

Essa operação deve ser administrativa, autenticada, autorizada e auditada.

### Revogar objeto

```http
POST /pontemesh/objects/{objectId}/revoke
```

Revoga novas autorizações para o objeto.

A revogação deve impedir emissão de novos pacotes de acesso e remover o objeto de fontes elegíveis conforme política aplicável.

Na implementação atual, a revogação administrativa é:

```http
POST /api/admin/buckets/{bucket}/object-revocations/{objectKey}
```

Ela marca o objeto ativo como `REVOKED`. O objeto continua no catálogo para
auditoria e rastreabilidade, mas deixa de ser servido pelo Origin, deixa de
aparecer em sync-plan de réplica e não pode receber novos manifestos ou pacotes
de acesso.

### Revogar pacote de acesso

```http
POST /pontemesh/access-packages/{accessPackageId}/revoke
```

Revoga um pacote de acesso específico.

Após revogação, o pacote não deve ser aceito em novas operações e pode exigir revalidação por parte do SDK em transferências prolongadas.

### Reportar métricas do SDK

```http
POST /pontemesh/sdk/metrics
```

Permite que SDKs reportem métricas operacionais, como:

* bytes obtidos por fonte;
* fragmentos validados;
* fragmentos inválidos;
* falhas por fonte;
* eventos de fallback;
* tempo de download;
* tempo até primeiro uso;
* tentativas por fragmento.

Essas métricas devem ser usadas para observabilidade e avaliação da redução de carga no Origin.

## API de Replica/Edge

A API de Replica/Edge deve permitir que réplicas autorizadas se registrem, sincronizem conteúdos e anunciem disponibilidade.

Replica/Edge não deve atuar como autoridade independente. Toda operação deve ser autenticada, autorizada, auditável e revogável pelo Origin.

### Responsabilidades

A API de Replica/Edge deve permitir:

* registrar identidade de réplica;
* autenticar requisições entre Origin e Replica/Edge;
* validar escopos da réplica;
* obter plano de sincronização autorizado;
* baixar objetos ou fragmentos autorizados;
* anunciar disponibilidade de fragmentos;
* reportar métricas de saúde;
* reportar métricas de transferência;
* receber revogações;
* receber mudanças de política.

### Registrar Replica/Edge

```http
POST /pontemesh/replicas/register
```

Registra uma réplica no Origin.

Essa operação deve exigir autenticação administrativa ou fluxo seguro de provisionamento.

Cada réplica deve possuir identidade própria e credenciais específicas.

### Obter plano de sincronização

```http
GET /pontemesh/replicas/{replicaId}/sync-plan
```

Retorna o plano de sincronização autorizado para uma réplica.

O plano pode indicar:

* buckets permitidos;
* objetos permitidos;
* fragmentos permitidos;
* prioridade de sincronização;
* validade da autorização;
* limites de banda;
* limites de armazenamento;
* política de retenção;
* revogações pendentes.

### Baixar fragmento autorizado

```http
GET /pontemesh/replicas/{replicaId}/fragments/{fragmentId}
```

Permite que a réplica baixe fragmentos autorizados a partir do Origin.

A operação deve validar identidade, escopo, expiração e política aplicável.

### Anunciar disponibilidade

```http
POST /pontemesh/replicas/{replicaId}/availability
```

Permite que a réplica anuncie quais objetos ou fragmentos possui localmente.

O Origin não deve confiar cegamente nesse anúncio. A disponibilidade anunciada deve ser auditável e pode ser validada conforme política.

### Reportar saúde

```http
POST /pontemesh/replicas/{replicaId}/health
```

Permite que a réplica informe estado operacional.

Pode incluir:

* disponibilidade;
* uso de armazenamento;
* uso de banda;
* latência com o Origin;
* erros recentes;
* versão do software;
* estado de sincronização.

### Reportar métricas de transferência

```http
POST /pontemesh/replicas/{replicaId}/metrics
```

Permite registrar métricas de uso da réplica.

Pode incluir:

* bytes servidos;
* fragmentos servidos;
* objetos atendidos;
* falhas de autenticação;
* falhas de autorização;
* falhas de sincronização;
* revogações aplicadas.

### Revogar Replica/Edge

```http
POST /pontemesh/replicas/{replicaId}/revoke
```

Revoga uma réplica.

Após revogação, a réplica deve ser removida das fontes elegíveis e não deve receber novos planos de sincronização.

## API de Observabilidade

A API de Observabilidade deve permitir acompanhar saúde, métricas e auditoria do Origin.

### Saúde do serviço

```http
GET /health
```

Retorna o estado básico do serviço.

Pode indicar:

* status da aplicação;
* conectividade com armazenamento;
* conectividade com banco de dados;
* estado de componentes internos;
* versão da aplicação.

### Métricas

```http
GET /metrics
```

Expõe métricas operacionais do Origin.

Pode incluir:

* bytes servidos pelo Origin;
* bytes servidos por Replica/Edge;
* bytes servidos por peers;
* taxa de fallback;
* tempo médio de download;
* tempo até primeiro uso;
* fragmentos invalidados;
* pacotes de acesso emitidos;
* pacotes de acesso negados;
* objetos revogados;
* réplicas ativas;
* réplicas revogadas.

### Auditoria

```http
GET /pontemesh/audit-events
```

Permite consultar eventos de auditoria.

Eventos auditáveis incluem:

* emissão de pacote de acesso;
* negação de pacote de acesso;
* revogação;
* deleção lógica;
* alteração de política;
* registro de réplica;
* sincronização de réplica;
* falha de autenticação;
* falha de autorização;
* operação administrativa sensível;
* eventos MCP, quando existir.

A consulta de auditoria deve ser protegida por autenticação, autorização e escopo administrativo.

## API administrativa

A API administrativa deve ser usada por painel, ferramentas internas e integrações futuras.

Ela deve permitir controlar recursos que não pertencem naturalmente ao modelo S3-like.

Responsabilidades possíveis:

* gerenciar políticas;
* gerenciar Replica/Edge;
* consultar métricas;
* consultar auditoria;
* revogar objetos, usuários, aplicações e réplicas;
* configurar estratégias de fallback;
* configurar priorização de fragmentos;
* configurar limites operacionais;
* configurar comportamento de buckets e objetos;
* preparar integração futura com MCP.

Toda operação administrativa deve ser autenticada, autorizada e auditada.

## Regras de segurança

As APIs do Origin devem seguir as seguintes regras:

* endpoints de controle exigem autenticação e autorização;
* operações administrativas exigem escopo administrativo explícito;
* pacotes de acesso devem ser emitidos apenas pelo Origin;
* manifestos devem ser protegidos contra adulteração;
* fragmentos devem ser validados por hash pelo SDK;
* Replica/Edge deve possuir identidade própria;
* Replica/Edge deve ser autenticada e autorizada pelo Origin;
* Replica/Edge deve ser revogável;
* peers não devem ser considerados confiáveis sem validação;
* URLs temporárias, tickets e pacotes de acesso devem expirar;
* operações sensíveis devem ser auditadas;
* configurações ausentes ou ambíguas devem negar acesso.

A implementação deve usar bibliotecas e frameworks consolidados para autenticação, autorização, assinatura, tokens, criptografia, mTLS, validação de JWT, hashing e comparação segura de assinaturas. O projeto não deve implementar mecanismos próprios de segurança quando existirem alternativas maduras.

## Regras de resposta e erro

As APIs devem retornar erros consistentes e seguros.

Exemplos conceituais:

* `400 Bad Request`: requisição malformada ou parâmetros inválidos;
* `401 Unauthorized`: autenticação ausente ou inválida;
* `403 Forbidden`: autenticação válida, mas sem autorização suficiente;
* `404 Not Found`: recurso inexistente ou não visível para o solicitante;
* `409 Conflict`: conflito de estado, versão ou política;
* `410 Gone`: objeto removido logicamente ou não mais disponível, quando a política permitir revelar esse estado;
* `416 Range Not Satisfiable`: intervalo de bytes inválido;
* `429 Too Many Requests`: limite de taxa excedido;
* `500 Internal Server Error`: falha interna inesperada;
* `503 Service Unavailable`: serviço temporariamente indisponível.

As respostas de erro não devem vazar segredos, tokens, existência de objetos protegidos ou detalhes internos sensíveis.

## Relação entre S3-like e Ponte Mesh

A API S3-like deve permanecer focada nas operações fundamentais de buckets e objetos.

A API Ponte Mesh deve concentrar os recursos específicos da arquitetura híbrida.

Exemplos de recursos que pertencem à API Ponte Mesh:

* pacote de acesso;
* manifesto autorizado;
* seleção de fontes;
* fallback;
* Replica/Edge;
* métricas específicas;
* auditoria;
* políticas de distribuição;
* priorização de fragmentos;
* revalidação de autorização;
* contratos para SDK.

Essa separação evita distorcer a API S3-like e preserva liberdade para implementar funcionalidades específicas do Ponte Mesh.

## Síntese

A API do Origin deve permitir que o servidor atue como autoridade central da arquitetura.

Ela deve oferecer uma camada S3-like para operações fundamentais de objetos e buckets, além de APIs próprias do Ponte Mesh para manifestos, pacotes de acesso, políticas, Replica/Edge, fallback, métricas e auditoria.

O Origin deve continuar funcional mesmo sem peers ou réplicas disponíveis. Fontes auxiliares são otimizações controladas, não dependências obrigatórias para o servidor cumprir sua finalidade.
