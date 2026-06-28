# API

O servidor deve expor dois grupos principais de API:

* **API S3-like**, voltada à integração familiar com buckets e objetos.
* **API Ponte Mesh**, voltada a manifestos, pacotes de acesso, réplicas, métricas, políticas, operação e configurações específicas da arquitetura híbrida.

Este documento é conceitual. Os contratos finais devem preservar os requisitos de segurança definidos em `docs/SECURITY.md`.

## Diretriz geral

A API S3-like deve ser utilizada como base para as operações fundamentais de armazenamento e recuperação de objetos.

Isso significa que operações essenciais, como criação de buckets, envio de objetos, leitura de objetos, consulta de metadados, remoção lógica e recuperação por intervalo de bytes, devem buscar compatibilidade com o modelo S3 sempre que possível.

Entretanto, a arquitetura do Ponte Mesh não deve ficar limitada às capacidades nativas da API S3.

Existem configurações, políticas e comportamentos próprios da distribuição híbrida que não possuem representação direta no contrato S3, como seleção de fontes, prioridades de fragmentos, políticas de fallback, configuração de Replica/Edge, estratégias de obtenção progressiva, métricas operacionais e regras específicas de autorização.

Nesses casos, o servidor tem liberdade para expor APIs próprias do Ponte Mesh, separadas da API S3-like, desde que a separação de responsabilidades seja preservada.

Em resumo:

* operações base de objeto devem passar preferencialmente pela API S3-like;
* configurações avançadas e comportamentos específicos da arquitetura híbrida devem ser expostos pela API Ponte Mesh;
* a API S3-like não deve ser distorcida para representar conceitos que pertencem ao domínio específico do Ponte Mesh;
* o dashboard administrativo futuro poderá utilizar as APIs próprias do Ponte Mesh para configurar políticas, réplicas, métricas, estratégias e parâmetros operacionais.

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

Externamente, uma aplicação deve conseguir trocar um endpoint S3 tradicional por um endpoint Origin do Ponte Mesh quando utilizar o subconjunto suportado.

Internamente, porém, o Origin pode aplicar regras próprias da arquitetura, como geração de manifesto, autorização, fragmentação, seleção de fontes, validação de integridade e fallback.

A compatibilidade S3-like deve ser entendida como uma interface de entrada familiar para operações comuns de objeto, não como uma limitação arquitetural.

## Limites da API S3-like

Nem toda funcionalidade do Ponte Mesh deve ser forçada dentro da API S3-like.

A API S3 não foi projetada para expressar todos os comportamentos necessários em uma arquitetura de distribuição híbrida controlada por Origin, com SDK, Replica/Edge, peers autorizados, manifestos, fragmentos, fallback adaptativo e políticas específicas de obtenção.

Portanto, recursos que não se encaixarem naturalmente no modelo S3 devem ser tratados por APIs próprias do Ponte Mesh.

Exemplos de funcionalidades que podem exigir APIs específicas:

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

Essas APIs devem complementar a API S3-like, não substituí-la nas operações fundamentais de objeto.

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

## Políticas e configurações avançadas

As políticas específicas da arquitetura devem ser representadas por contratos próprios do Ponte Mesh.

Essas políticas podem controlar, por exemplo:

* se um objeto pode ou não ser distribuído por P2P;
* se um bucket permite Replica/Edge;
* se um conteúdo deve priorizar obtenção sequencial;
* se fragmentos iniciais devem ser priorizados para consumo progressivo;
* se fragmentos raros devem ter prioridade;
* qual limite de falhas aciona fallback;
* quando uma sessão deve migrar completamente para o Origin;
* quais fontes podem ser usadas em cada contexto;
* quais métricas devem ser coletadas durante a transferência;
* quais eventos devem ser auditados.

Essas configurações não pertencem naturalmente ao modelo S3 e, por isso, devem ser modeladas na API Ponte Mesh.

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

O Replica/Edge não deve atuar como autoridade independente. Ele deve operar dentro das regras emitidas pelo Origin e respeitar políticas de autorização, expiração e revogação.

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

O SDK deve usar a API S3-like para operações base quando adequado, mas pode utilizar APIs específicas do Ponte Mesh para comportamentos avançados que não cabem no modelo S3.

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

A existência da API S3-like não impede a criação dessas APIs administrativas. Pelo contrário, a separação permite manter a compatibilidade com operações familiares de objeto enquanto preserva liberdade arquitetural para controlar recursos específicos do Ponte Mesh.

## Síntese

A API S3-like deve ser usada como contrato familiar para operações essenciais de buckets e objetos.

A API Ponte Mesh deve ser usada para tudo que ultrapassar o modelo S3, incluindo políticas, manifestos, fragmentação, fontes autorizadas, fallback, Replica/Edge, métricas, auditoria e configurações avançadas.

Essa separação evita distorcer a API S3-like e permite que o Origin continue oferecendo uma interface conhecida para integração, sem abrir mão dos recursos específicos necessários para a distribuição híbrida proposta pelo Ponte Mesh.