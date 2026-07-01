# MCP administrativo

O Ponte Mesh Server expõe uma interface MCP sobre o plano de controle administrativo.

MCP não participa do plano de dados, não transfere fragmentos e não substitui as
autorizações emitidas pelo Origin. Ele existe para automação administrativa,
consulta operacional, backup de configuração e integração com clientes compatíveis
com Model Context Protocol.

## Endpoint

Na implementação atual, o endpoint MCP usa Streamable HTTP com JSON-RPC:

```http
POST /mcp
```

O endpoint é desabilitado por padrão. A configuração é feita pelo painel ou pelas
rotas administrativas:

```http
GET /api/admin/mcp/settings
PUT /api/admin/mcp/settings
GET /api/admin/mcp/status
GET /api/admin/mcp/tokens
POST /api/admin/mcp/tokens
DELETE /api/admin/mcp/tokens/{id}
GET /api/admin/mcp/activity
```

Essas rotas exigem sessão administrativa.

## Segurança

MCP mantém autenticação obrigatória. A configuração não permite desabilitar
`requireAuth`.

Controles aplicados:

* endpoint desabilitado por padrão;
* token MCP próprio, separado de credenciais S3, aplicação e réplica;
* token exibido somente uma vez na criação;
* revogação de tokens;
* opção para permitir apenas origem localhost;
* rate limit por token;
* auditoria de chamadas, erros e rejeições;
* ferramentas de escrita desabilitadas por padrão.

## Métodos JSON-RPC suportados

```text
initialize
notifications/initialized
ping
tools/list
tools/call
resources/list
resources/read
prompts/list
prompts/get
```

## Ferramentas de leitura

```text
pontemesh_get_instance_status
pontemesh_get_storage_summary
pontemesh_list_buckets
pontemesh_get_bucket
pontemesh_list_objects
pontemesh_get_object_metadata
pontemesh_get_health
pontemesh_get_recent_audit_events
pontemesh_export_configuration
```

`pontemesh_export_configuration` retorna configurações operacionais sem segredos:
settings MCP e políticas de buckets.

## Ferramentas de escrita

As ferramentas de escrita aparecem somente quando `writeToolsEnabled` está ativo:

```text
pontemesh_update_bucket_policy
pontemesh_import_configuration
```

Essas ferramentas usam os mesmos validadores do catálogo e não podem contornar as
regras normais do Origin. Importação de configuração não cria buckets e não importa
tokens, access keys ou segredos.

## Recursos

```text
pontemesh://instance/status
pontemesh://instance/health
pontemesh://storage/summary
pontemesh://buckets
pontemesh://buckets/{bucket}
pontemesh://buckets/{bucket}/objects
pontemesh://audit/recent
```

## Limites

MCP é uma superfície administrativa. Operações de download, upload, sincronização de
fragmentos e tráfego de objeto continuam nos endpoints S3-compatible e Ponte Mesh.
