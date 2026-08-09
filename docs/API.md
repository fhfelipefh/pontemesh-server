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

### Estado do setup

O painel consulta o estado público da configuração inicial em:

```http
GET /api/setup/status
```

A resposta informa se o setup ainda é necessário e a versão exata do binário em
execução:

```json
{
  "setupRequired": true,
  "serverVersion": "0.2.2",
  "internalWebPort": 8080,
  "internalS3Port": 9000,
  "publicWebUrl": "https://origin.example.com",
  "publicS3Url": "https://origin.example.com:9443"
}
```

`serverVersion` vem da versão compilada do servidor e deve ser exibida
discretamente em todas as etapas da configuração inicial.
As portas retornadas são listeners internos já usados pelo processo, não portas
HTTPS públicas que o usuário deva escolher durante o setup.
Quando o operador definiu os endpoints públicos por variáveis de ambiente, o
status também os retorna para que o formulário seja preenchido automaticamente.
Esses valores são endereços públicos e não contêm credenciais.

Ao concluir o setup de um Origin, `publicWebUrl` e `publicS3Url` podem persistir
os dois endpoints externos. Eles precisam encaminhar, respectivamente, ao
listener web e ao listener S3-compatible. O endpoint S3 normalmente usa outro
host ou outra porta TLS quando existe proxy reverso. Um Origin precisa anunciar
um endpoint S3 alcançável para que SDKs e aplicações consigam baixar objetos.

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

A listagem de objetos aceita `ListObjects` v1, usado por clientes como o
WinSCP, e `ListObjectsV2`. A versão v1 pagina por `marker`; a versão v2 usa
`continuation-token` ou `start-after`.

Na implementação atual, o painel administrativo e a API S3-compatible ficam em
portas separadas. O painel web/admin usa `http://localhost:8080`; a API
S3-compatible usa `http://localhost:9000` e expõe as operações S3 na raiz dessa porta.

Essas rotas exigem credenciais S3 próprias com AWS Signature Version 4.

Externamente, uma aplicação deve conseguir trocar um endpoint S3 tradicional por um endpoint Origin do Ponte Mesh quando utilizar o subconjunto suportado.

Internamente, o Origin aplica autorização, catálogo, versionamento, auditoria e prepara metadados para manifestos e distribuição híbrida.

A compatibilidade S3-like deve ser entendida como uma interface de entrada familiar para operações comuns de objeto, não como uma limitação arquitetural.

Na implementação atual, a API S3-compatible roda em listener próprio, por padrão
em `:9000`, e expõe buckets e objetos na raiz do endpoint. O painel web/admin
permanece separado, por padrão em `:8080`.

Pacotes de acesso emitidos pelo Origin são consumidos por rotas próprias da API
Ponte Mesh em `/pontemesh/access-packages/...`. Esse fluxo atende SDKs e usa
autorização temporária separada das credenciais S3.

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

Na implementação atual, estão disponíveis:

```http
POST /pontemesh/access-packages
POST /pontemesh/access-packages/{packageId}/revalidate/{bucket}/{objectKey}
GET /pontemesh/access-packages/{packageId}/objects/{bucket}/{objectKey}
GET /pontemesh/objects/{bucket}/manifest/{objectKey}
GET /pontemesh/objects/{bucket}/sources/{objectKey}
GET /pontemesh/replicas/{replicaId}/sync-plan
GET /pontemesh/replicas/{replicaId}/objects/{bucket}/{objectKey}
GET /pontemesh/replicas/{replicaId}/manifests/{manifestId}/fragments/{fragmentId}
POST /pontemesh/replicas/{replicaId}/availability
POST /pontemesh/replicas/{replicaId}/health
POST /pontemesh/replicas/{replicaId}/metrics
GET /pontemesh/replicas/{replicaId}/policy-updates
```

Os endpoints de aplicação/SDK exigem `Authorization: Bearer <token>` de aplicação. Endpoints de Replica/Edge exigem credencial própria de réplica, assinatura da requisição, timestamp e nonce.
O pacote de acesso autoriza fontes elegíveis reais, sempre com fallback para
Origin e inclusão do manifesto gerado pelo Origin.

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

## Identidade administrativa da instância

O painel administrativo consulta e pode alterar o nome exibido da instância por
meio de um contrato autenticado:

```http
GET /api/admin/instance
PUT /api/admin/instance
Content-Type: application/json

{ "name": "Ponte Mesh Origin" }
```

O `PUT` aceita um nome sem caracteres de controle, com 1 a 100 caracteres após
a remoção de espaços externos. A alteração persiste somente o nome em
`config.toml`, preserva papel, endpoints e armazenamento, exige sessão
administrativa e gera evento de auditoria.

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
GET /api/admin/configuration
POST /api/admin/configuration
GET /api/admin/mcp/settings
PUT /api/admin/mcp/settings
GET /api/admin/mcp/status
GET /api/admin/mcp/tokens
POST /api/admin/mcp/tokens
DELETE /api/admin/mcp/tokens/{id}
GET /api/admin/mcp/activity
GET /api/admin/metrics/origin-traffic
GET /api/admin/metrics/replica-traffic
GET /api/admin/metrics/buckets
GET /api/admin/metrics/objects
GET /api/admin/metrics/replicas/{replicaId}
GET /api/admin/buckets/{bucket}/policy
PUT /api/admin/buckets/{bucket}/policy
GET /api/admin/bucket-policy-defaults
PUT /api/admin/bucket-policy-defaults
PUT /api/admin/buckets/bulk-policy
GET /api/admin/application-credentials
POST /api/admin/application-credentials
POST /api/admin/application-credentials/{id}/revoke
POST /api/admin/access-packages/{packageId}/revoke
GET /api/admin/replicas
POST /api/admin/replicas
POST /api/admin/replicas/{replicaId}/revoke
POST /api/admin/buckets/{bucket}/object-revocations/{objectKey}
```

Essas rotas exigem sessão administrativa do painel.

Os padrões da instância abrangem somente opções próprias da distribuição
híbrida do Ponte Mesh. Novos buckets recebem uma cópia desses valores. A edição
em massa aplica o mesmo conjunto a todos os buckets ou a uma lista explícita,
preservando opções S3 e políticas específicas que não façam parte do pedido.

`GET /api/admin/configuration` exporta configurações operacionais sem segredos,
incluindo settings MCP e políticas de buckets. `POST /api/admin/configuration`
importa o mesmo formato e aplica apenas políticas de buckets existentes.

MCP também está disponível como interface administrativa do plano de controle em
`POST /mcp`. O contrato detalhado está em `docs/api/mcp.md`.

## Síntese

A API S3-like deve ser usada como contrato familiar para operações essenciais de buckets e objetos.

A API Ponte Mesh deve ser usada para tudo que ultrapassar o modelo S3, incluindo políticas, manifestos, fragmentação, fontes autorizadas, fallback, Replica/Edge, métricas, auditoria e configurações avançadas.

Essa separação mantém a API S3-compatible familiar e preserva os contratos específicos da distribuição híbrida.
