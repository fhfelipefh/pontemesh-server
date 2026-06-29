# API

O servidor deve expor dois grupos principais de API:

* **API S3-like**, voltada à integração familiar com buckets e objetos.
* **API Ponte Mesh**, voltada a manifestos, pacotes de acesso, réplicas, métricas, políticas, operação e configurações específicas da arquitetura híbrida.

Este documento é conceitual. Os contratos finais devem preservar os requisitos de segurança definidos em `docs/SECURITY.md`.

## Diretriz geral

A API S3-like cobre operações fundamentais de armazenamento e recuperação de objetos. A API Ponte Mesh cobre políticas, manifestos, pacotes de acesso, Replica/Edge, métricas, auditoria e demais recursos da distribuição híbrida.

Em resumo:

* operações base de objeto passam pela API S3-like;
* configurações avançadas e comportamentos híbridos pertencem à API Ponte Mesh;
* o dashboard administrativo usa APIs próprias para políticas, réplicas, métricas, estratégias e parâmetros operacionais.

## API S3-like mínima

A API S3-like deve oferecer um subconjunto mínimo de operações familiares ao ecossistema S3:

* criar e listar buckets;
* enviar objeto;
* listar objetos;
* consultar metadados por `HEAD`;
* recuperar objeto;
* recuperar intervalos de bytes;
* remover logicamente objeto;
* gerar URL temporária ou mecanismo equivalente.

Na implementação atual, o painel administrativo e a API S3-compatible ficam em
portas separadas. O painel web/admin usa `http://localhost:8080`; a API
S3-compatible usa `http://localhost:9000` e expõe as operações S3 na raiz dessa porta.

Essas rotas exigem credenciais S3 próprias com AWS Signature Version 4.

Externamente, uma aplicação deve conseguir trocar um endpoint S3 tradicional por um endpoint Origin do Ponte Mesh quando utilizar o subconjunto suportado.

Internamente, o Origin aplica autorização, catálogo, versionamento, auditoria e prepara metadados para manifestos e distribuição híbrida.

A compatibilidade S3-like deve ser entendida como uma interface de entrada familiar para operações comuns de objeto, não como uma limitação arquitetural.

## Recursos da API Ponte Mesh

Recursos de distribuição híbrida ficam em APIs próprias:

* definir se a obtenção deve priorizar cabeçalhos, fragmentos iniciais ou fragmentos raros;
* configurar estratégias como `headers-first`, `priority-first`, `rarest-first` ou políticas equivalentes;
* definir limites de falha antes de acionar fallback;
* configurar pesos de seleção de fonte;
* controlar políticas de Replica/Edge;
* habilitar ou desabilitar colaboração P2P por objeto, bucket, usuário ou aplicação;
* definir regras de expiração e revogação específicas;
* consultar métricas de peers, réplicas e Origin;
* auditar fontes utilizadas em uma transferência;
* consultar estado de disponibilidade de fragmentos;
* configurar políticas futuras utilizadas pelo dashboard administrativo.

Essas APIs complementam a API S3-like nas operações fundamentais de objeto.

## API Ponte Mesh

A API Ponte Mesh concentra os contratos específicos da arquitetura híbrida.

Responsabilidades esperadas:

* obter pacote de acesso para um objeto;
* consultar manifesto autorizado;
* consultar estado de disponibilidade;
* revogar acesso, objeto, usuário, aplicação ou réplica;
* registrar ou anunciar disponibilidade de réplica;
* sincronizar réplica a partir do Origin;
* consultar métricas e auditoria;
* fornecer contratos para SDKs;
* configurar políticas de obtenção híbrida;
* configurar estratégias de priorização de fragmentos;
* configurar regras de fallback;
* configurar limites operacionais para peers e réplicas;
* expor contratos administrativos para uso futuro por dashboard.

Na implementação inicial, estão disponíveis:

```http
POST /pontemesh/access-packages
GET /pontemesh/objects/{bucket}/manifest/{objectKey}
GET /pontemesh/replicas/{replicaId}/sync-plan
```

Os endpoints de aplicação/SDK exigem `Authorization: Bearer <token>` de aplicação. O endpoint de sync-plan exige credencial própria de Replica/Edge.
O pacote de acesso inicial autoriza somente a fonte `ORIGIN`, informa fallback
pelo endpoint S3-compatible `/{bucket}/{objectKey}` e embute o manifesto gerado
pelo Origin.

O manifesto e o pacote de acesso usam a política persistida do bucket para
definir tamanho de fragmento e TTL máximo do pacote.

## Políticas e configurações avançadas

As políticas específicas da arquitetura devem ser representadas por contratos próprios do Ponte Mesh.

Essas políticas podem controlar, por exemplo:

* habilitação de distribuição por P2P;
* se um bucket permite Replica/Edge;
* se um conteúdo deve priorizar obtenção sequencial;
* se fragmentos iniciais devem ser priorizados para consumo progressivo;
* se fragmentos raros devem ter prioridade;
* qual limite de falhas aciona fallback;
* quando uma sessão deve migrar completamente para o Origin;
* quais fontes podem ser usadas em cada contexto;
* quais métricas devem ser coletadas durante a transferência;
* quais eventos devem ser auditados.

Essas configurações pertencem à API Ponte Mesh.

## Contratos para Replica/Edge

Toda chamada de réplica deve ser autenticada e autorizada.

A API de réplica deve permitir:

* registrar identidade de réplica;
* autenticar requisições entre Origin e Replica;
* obter plano de sincronização autorizado;
* baixar objeto ou fragmentos autorizados do Origin;
* anunciar disponibilidade de fragmentos;
* reportar métricas e saúde;
* receber revogações e mudanças de política.

Replica/Edge opera dentro das regras emitidas pelo Origin e respeita políticas de autorização, expiração e revogação.

## Contratos para SDKs

A API Ponte Mesh também deve fornecer contratos adequados para SDKs.

O SDK deve conseguir:

* solicitar pacote de acesso;
* obter manifesto autorizado;
* consultar fontes autorizadas;
* receber políticas de seleção de fragmentos;
* receber políticas de seleção de fontes;
* consultar endpoints de fallback;
* reportar progresso, falhas e métricas;
* revalidar autorização durante transferências prolongadas;
* informar fragmentos disponíveis para compartilhamento temporário quando permitido.

O SDK usa a API S3-like para operações base e APIs Ponte Mesh para manifestos, pacotes de acesso, fontes e políticas.

## Relação com o dashboard

O dashboard administrativo futuro deve utilizar principalmente as APIs próprias do Ponte Mesh.

Por meio dessas APIs, o dashboard poderá configurar e consultar:

* buckets e objetos;
* políticas de distribuição;
* permissões;
* Replica/Edge;
* métricas;
* auditoria;
* revogações;
* estratégias de fallback;
* priorização de fragmentos;
* estados de disponibilidade;
* comportamento dos SDKs.

Na implementação atual, o plano administrativo já expõe contratos para:

```http
GET /api/admin/audit-events
GET /api/admin/metrics/origin-traffic
GET /api/admin/buckets/{bucket}/policy
PUT /api/admin/buckets/{bucket}/policy
GET /api/admin/application-credentials
POST /api/admin/application-credentials
GET /api/admin/replicas
POST /api/admin/replicas
POST /api/admin/replicas/{replicaId}/revoke
POST /api/admin/buckets/{bucket}/object-revocations/{objectKey}
```

Essas rotas exigem sessão administrativa do painel.

## Síntese

A API S3-like deve ser usada como contrato familiar para operações essenciais de buckets e objetos.

A API Ponte Mesh deve ser usada para tudo que ultrapassar o modelo S3, incluindo políticas, manifestos, fragmentação, fontes autorizadas, fallback, Replica/Edge, métricas, auditoria e configurações avançadas.

Essa separação mantém a API S3-compatible familiar e preserva os contratos específicos da distribuição híbrida.
