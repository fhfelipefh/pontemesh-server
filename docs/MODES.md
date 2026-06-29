# Papéis de operação

Este documento descreve os papéis operacionais previstos para o `pontemesh-server`.

A arquitetura considera dois papéis principais:

* **Origin**;
* **Replica/Edge**.

Quando não há fontes auxiliares elegíveis, o **Origin** serve objetos diretamente e preserva as mesmas regras de autenticação, autorização, manifesto, integridade, métricas e revogação.

## Origin

O **Origin** é o papel principal do servidor.

Ele atua como autoridade do plano de controle e como fonte direta ou fonte final de garantia no plano de dados.

Responsabilidades principais:

* manter o catálogo de buckets, objetos, versões e metadados;
* receber e armazenar objetos;
* expor o subconjunto de API S3-like para operações fundamentais de objeto;
* expor APIs próprias do Ponte Mesh para recursos que não cabem naturalmente no modelo S3;
* autenticar usuários, aplicações, SDKs e réplicas;
* autorizar previamente toda obtenção de conteúdo;
* emitir pacotes de acesso;
* gerar ou disponibilizar manifestos;
* controlar políticas de bucket e objeto;
* controlar expiração e revogação;
* registrar e validar fontes autorizadas;
* fornecer endpoints de fallback;
* servir objetos ou fragmentos diretamente quando necessário;
* registrar métricas e eventos de auditoria;
* fornecer contratos estáveis para SDKs e Replica/Edge.

O Origin deve participar do início de toda obtenção. Nenhum SDK, peer ou Replica/Edge deve obter ou servir conteúdo sem autorização prévia emitida pelo Origin.

Na entrega direta, o Origin mantém o controle centralizado da operação.

## Replica/Edge

O **Replica/Edge** é um papel auxiliar do servidor.

Ele atua como fonte mais estável no plano de dados, replicando conteúdos autorizados a partir do Origin e servindo fragmentos conforme escopos, políticas e autorizações definidos pelo Origin.

Responsabilidades principais:

* autenticar-se com o Origin usando identidade própria;
* operar com credenciais específicas de réplica;
* receber escopos explícitos de atuação;
* obter planos de sincronização autorizados;
* sincronizar objetos ou fragmentos a partir do Origin;
* armazenar localmente conteúdos autorizados;
* anunciar disponibilidade de fragmentos;
* servir fragmentos autorizados;
* reportar métricas de saúde, disponibilidade e transferência;
* receber e aplicar revogações;
* respeitar mudanças de política emitidas pelo Origin;
* registrar eventos relevantes para auditoria.

Replica/Edge distribui apenas conteúdo autorizado pelas políticas emitidas pelo Origin. A autoridade continua no Origin.

## Relação entre Origin e Replica/Edge

A comunicação entre **Origin** e **Replica/Edge** deve ser:

* autenticada;
* autorizada;
* auditável;
* revogável;
* restrita por escopos;
* compatível com políticas de expiração e revogação.

O Replica/Edge existe para reforçar disponibilidade e reduzir carga do Origin em cenários apropriados, mas não substitui o Origin como autoridade central.

Se uma réplica estiver indisponível, expirada, revogada ou fora da política aplicável, o SDK deve ignorá-la como fonte elegível e recorrer a outra fonte autorizada, incluindo o próprio Origin quando necessário.

## Ausência de fontes auxiliares

A arquitetura não depende obrigatoriamente da existência de peers ou réplicas para funcionar.

Quando não houver peers autorizados, Replica/Edge disponível ou fonte auxiliar tecnicamente vantajosa, a obtenção deve ocorrer diretamente pelo Origin.

Esse comportamento não exige modo especial. Trata-se do fluxo normal de fallback e garantia da arquitetura.

O objetivo do Ponte Mesh é reduzir carga do Origin quando houver condições seguras e vantajosas para distribuição híbrida, não eliminar a função do Origin.

## Síntese

O `pontemesh-server` deve operar em um dos seguintes papéis:

* **Origin**, como autoridade central, armazenamento primário, emissor de autorizações, gerador de manifestos e fonte final de garantia;
* **Replica/Edge**, como nó auxiliar autorizado, responsável por replicar e servir fragmentos conforme políticas emitidas pelo Origin.

A entrega direta pelo Origin sem peers ou réplicas é um comportamento normal da arquitetura, não um modo separado.
