# Ponte Mesh Server/Replica - Base de conhecimento do repositorio

Este repositorio representa o servidor Origin e o modo Replica/Edge do Ponte Mesh,
baseado na proposta de um framework de alto nivel para distribuicao hibrida de
conteudo com abstracao de peer-to-peer e fallback por servidor.

## Regra de escopo

- Não implementar codigo sem pedido explicito.
- Antes de programar, manter a documentacao alinhada a proposta arquitetural e as
  decisoes de seguranca.
- Este repositorio e o lado servidor/replica. O SDK e os clientes sao componentes
  externos que consomem os contratos produzidos pelo Origin.
- Seguranca é um requisito central, não detalhe posterior.

## Objetivo do projeto

O Ponte Mesh propoe um framework de distribuicao hibrida de objetos digitais. O
Origin mantem ingestao, catalogo, autenticacao, autorizacao, manifesto, revogacao,
politicas e fallback. O plano de dados pode usar Origin, Replica/Edge e peers
autorizados para transferir fragmentos, mas toda obtencao comeca com consulta e
autorizacao do Origin.

O sistema busca reduzir carga e custo do servidor central usando P2P e replicas
quando tecnicamente vantajoso, sem abrir mao de controle centralizado, previsibilidade,
integridade e revogacao.

## Componentes conceituais

- Origin: autoridade central. Controla armazenamento primario, catalogo, ingestao,
  autenticacao, autorizacao, pacotes de acesso, manifestos, politicas, revogacao,
  metricas e fallback.
- Replica/Edge: no servidor auxiliar mais estavel que replica objetos ou fragmentos
  a partir do Origin e serve como fonte de dados autorizada. Não e confiavel por
  presuncao: precisa autenticar-se com o Origin e receber autorizacao explicita para
  sincronizar, anunciar e servir conteudo.
- SDK: biblioteca externa usada por aplicacoes cliente. Consulta o Origin, interpreta
  manifestos, seleciona fontes, valida hashes, preserva progresso por fragmento e
  aplica fallback.
- Client: aplicacao consumidora. Pode compartilhar fragmentos temporariamente apenas
  se autorizado por politica do Origin.

## Principios obrigatorios

1. O Origin e o plano de controle.
2. Nenhum participante baixa, replica, anuncia ou serve conteudo sem autorizacao valida.
3. Replica/Edge deve autenticar a comunicacao com o Origin.
4. O plano de dados e desconfiado: todo fragmento recebido de peer ou Replica/Edge
   deve ser validado por hash antes de ser aceito.
5. Revogacao e expiracao bloqueiam novas autorizacoes e devem poder interromper
   transferencias longas apos revalidacao.
6. Fallback para Origin deve preservar fragmentos ja validados.
7. A API publica deve ser familiar para integracoes S3-like, mas o comportamento
   interno e hibrido.
8. MCP, quando existir, e interface administrativa do plano de controle, Não canal
   de dados.

## Seguranca server-origin <-> replica

A proposta define Replica/Edge como no auxiliar estavel, mas este repositorio deve
tratar explicitamente a comunicacao Origin <-> Replica como autenticada e autorizada.

Requisitos minimos:

- identidade propria para cada replica;
- credencial de replica separada de credenciais de usuario, cliente e SDK;
- autenticacao mutua ou assinatura forte de requisicoes entre Origin e Replica;
- escopo por replica: buckets/objetos permitidos, regioes, quotas, limites de banda
  e acoes permitidas;
- rotação e revogacao de credenciais;
- proteção contra replay usando timestamp, nonce, expiracao curta ou mecanismo
  equivalente;
- auditoria de sincronizacao, anuncio de disponibilidade, entrega de fragmentos e
  falhas de autenticacao;
- replicas revogadas devem parar de receber novas autorizacoes e deixar de ser
  anunciadas como fonte elegivel.

## Documentos principais

- `docs/ARCHITECTURE.md`: arquitetura conceitual do projeto.
- `docs/SECURITY.md`: modelo de ameacas e controles obrigatorios.
- `docs/REQUIREMENTS.md`: requisitos funcionais e Não funcionais.
- `docs/protocol/*.md`: contratos conceituais de manifesto, pacote de acesso,
  fragmentos, fallback, revogacao e selecao de fontes.
- `docs/operations/*.md`: operacao dos modos Origin, Replica/Edge e Standalone.
- `docs/decisions/*.md`: decisoes arquiteturais.

## O que evitar

- Não transformar Replica/Edge em bypass de autorizacao.
- Não confiar em hash enviado por peer/replica fora do manifesto assinado ou emitido
  pelo Origin.
- Não criar fluxo em que cliente consiga descobrir fontes ou fragmentos sem pacote
  de acesso valido.
- Não tratar delecao logica como garantia de apagamento fisico instantaneo em peers.
- Não acoplar a proposta a um unico protocolo P2P; o projeto reaproveita mecanismos
  consolidados e deixa o SDK abstrair transportes.
