# ADR 0001: Contratos iniciais do Origin

## Status

Superada pela separação do endpoint S3-compatible em porta própria.

## Contexto

O repositório já possui painel administrativo, setup inicial, sessão admin,
catálogo PostgreSQL e armazenamento local de objetos. A documentação conceitual
define o Origin como autoridade central e exige que obtenção, manifesto e pacote
de acesso comecem por autorização do Origin.

Como o painel web era servido no mesmo host e ocupava rotas como `/`,
`/dashboard` e `/buckets`, expor imediatamente o subconjunto S3-like na raiz
conflitaria com a experiência administrativa.

## Decisão

A implementação inicial usava:

* API S3-like co-hospedada no listener web;
* API Ponte Mesh sob o prefixo `/pontemesh`;
* credenciais operacionais de aplicação/SDK separadas da sessão admin;
* autenticação operacional por `Authorization: Bearer <token>`;
* token de aplicação exibido apenas no momento da criação;
* hash do token persistido no catálogo PostgreSQL;
* escopos explícitos para leitura, escrita, manifesto e pacote de acesso;
* fonte autorizada inicial limitada a `ORIGIN`;
* manifesto gerado pelo Origin a partir do objeto armazenado;
* pacote de acesso temporário, persistido no catálogo e emitido com TTL curto.

Essa decisão foi superada: o painel web/admin permanece em `:8080`, enquanto o
endpoint S3-compatible fica em `:9000` e expõe `/{bucket}/{objectKey}`.

## Consequências

* O painel continua acessível sem colisão de rotas.
* Nenhuma rota pública de objeto funciona sem credencial operacional válida.
* O Origin mínimo já consegue servir objeto completo e por `Range`.
* SDKs podem começar a consumir manifesto e pacote de acesso com `ORIGIN` como
  fonte final de garantia.
* Replica/Edge e peers continuam fora do plano de dados ativo até existir
  autenticação, autorização e revogação específicas.

## Escopo futuro

* manter o formato S3-like em endpoint dedicado;
* validar pacotes de acesso diretamente em chamadas de dados;
* assinar manifestos ou substituir o contrato por artefato JOSE equivalente;
* introduzir políticas por bucket/objeto;
* adicionar Replica/Edge somente após credenciais e escopos próprios.
