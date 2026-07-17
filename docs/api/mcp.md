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
* ferramentas de escrita desabilitadas por padrão;
* ferramentas administrativas desabilitadas por padrão;
* segredos existentes nunca são retornados por ferramentas de listagem ou exportação.

Segredos novos podem aparecer somente na resposta da operação que os criou, como
tokens MCP, credenciais de aplicação ou access keys S3. Depois disso, o servidor
mantém apenas hashes, metadados ou material cifrado conforme o tipo de credencial.

## Setup assistido por IA

O binário inclui um bootstrap local para preparar uma instância Origin e habilitar
MCP para clientes de IA:

```bash
pontemesh setup-agent
```

O comando usa `PONTEMESH_HOME` e `PONTEMESH_DATABASE_URL` do ambiente. Quando o
setup inicial ainda não foi concluído, ele cria a configuração Origin, o usuário
admin inicial, a access key S3 inicial e um token MCP. Quando a instância já está
configurada, ele apenas habilita MCP e cria um novo token MCP.

Por padrão, o token MCP gerado possui escopos `read,write,admin`, mas o endpoint
fica restrito a localhost e mantém autenticação obrigatória. O comando imprime um
JSON com os segredos recém-criados e grava a configuração MCP em:

```text
$PONTEMESH_HOME/secrets/setup-agent-mcp.json
```

Em sistemas Unix, esse arquivo é criado com permissão `0600`. Use
`--mcp-scopes read` ou `--mcp-scopes read,write` para reduzir permissões, e
`--allow-remote-mcp` somente quando houver controle de rede externo adequado.

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
pontemesh_get_ai_connection_guide
```

`pontemesh_export_configuration` retorna configurações operacionais sem segredos:
settings MCP e políticas de buckets.

`pontemesh_get_ai_connection_guide` retorna endpoints, método HTTP e orientação de
autenticação para clientes de IA sem incluir tokens existentes.

## Ferramentas de escrita

As ferramentas de escrita aparecem somente quando `writeToolsEnabled` está ativo:

```text
pontemesh_create_bucket
pontemesh_delete_bucket
pontemesh_put_text_object
pontemesh_put_base64_object
pontemesh_delete_object
```

Uploads via MCP são limitados a objetos pequenos. Arquivos grandes continuam sendo
responsabilidade da API S3-compatible.

## Ferramentas administrativas

As ferramentas administrativas aparecem somente quando `adminToolsEnabled` está
ativo e o token MCP possui escopo `admin`:

```text
pontemesh_update_bucket_policy
pontemesh_import_configuration
pontemesh_list_credentials
pontemesh_create_application_credential
pontemesh_create_s3_access_key
```

Essas ferramentas usam os mesmos validadores do catálogo e não podem contornar as
regras normais do Origin. Importação de configuração não cria buckets e não importa
tokens, access keys ou segredos.

`pontemesh_list_credentials` retorna somente metadados seguros. As ferramentas de
criação retornam o segredo uma única vez, na resposta de criação, e registram
auditoria.

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
