# Configuração

Este documento descreve a configuração conceitual do `pontemesh-server`.

Os arquivos `config/*.toml` ainda não definem a sintaxe final. A estrutura abaixo serve como referência arquitetural para orientar a implementação futura da configuração do servidor, das réplicas e das políticas operacionais.

## Blocos de configuração

A configuração deve ser organizada conforme o papel executado pelo componente dentro da arquitetura.

Os papéis previstos são:

* **Origin**;
* **Replica/Edge**.

A ausência de peers ou réplicas disponíveis não exige um modo especial de execução. Nesse cenário, o Origin continua operando normalmente como servidor central, entregando objetos diretamente e mantendo as mesmas regras de autenticação, autorização, manifesto, integridade, métricas e revogação.

## Origin

O bloco de configuração do **Origin** deve concentrar os parâmetros relacionados ao servidor central da arquitetura.

Configurações esperadas:

* endereço de escuta;
* armazenamento primário;
* banco de dados ou mecanismo de catálogo;
* chave de assinatura de manifestos;
* chave de assinatura de pacotes de acesso;
* políticas de bucket;
* políticas de objeto;
* expiração padrão de pacotes de acesso;
* limites para recuperação por intervalo de bytes;
* limites e regras de fallback;
* credenciais administrativas;
* configuração da API S3-like;
* configuração das APIs próprias do Ponte Mesh;
* registro de réplicas autorizadas;
* políticas de autorização para Replica/Edge;
* políticas de distribuição por peers autorizados;
* métricas;
* auditoria;
* parâmetros de revogação e expiração.

O Origin deve ser configurado como autoridade central do sistema. Ele é responsável por autenticar, autorizar, emitir pacotes de acesso, gerar manifestos, controlar disponibilidade e aplicar políticas de revogação.

Mesmo quando a distribuição híbrida estiver desabilitada, indisponível ou sem fontes auxiliares elegíveis, o Origin deve continuar capaz de atender às operações fundamentais de objeto.

## Replica/Edge

O bloco de configuração de **Replica/Edge** deve concentrar os parâmetros necessários para que uma réplica autorizada se comunique com o Origin e participe do plano de dados.

Configurações esperadas:

* identidade da réplica;
* endpoint do Origin;
* credencial, certificado ou chave da réplica;
* escopos permitidos;
* armazenamento local;
* política de sincronização;
* limites de banda;
* limites de armazenamento;
* intervalo de anúncio de disponibilidade;
* política de retenção local;
* comportamento ao receber revogação;
* comportamento ao receber mudanças de política;
* métricas de saúde;
* métricas de transferência;
* parâmetros de reconexão com o Origin.

A Replica/Edge não deve atuar como autoridade independente. Ela deve operar somente dentro dos escopos, políticas e autorizações definidos pelo Origin.

Toda comunicação entre Origin e Replica/Edge deve ser autenticada, autorizada, auditável e revogável.

## Políticas de distribuição

As configurações do Ponte Mesh podem incluir políticas específicas para controlar quando e como a distribuição híbrida será utilizada.

Essas políticas podem ser aplicadas em diferentes níveis, como:

* global;
* bucket;
* objeto;
* aplicação;
* usuário;
* réplica.

Exemplos de configurações possíveis:

* habilitar ou desabilitar distribuição por peers;
* habilitar ou desabilitar uso de Replica/Edge;
* definir prioridade entre Origin, Replica/Edge e peers;
* configurar limites de falha antes de acionar fallback;
* configurar expiração de pacotes de acesso;
* definir se a obtenção deve priorizar fragmentos iniciais;
* definir se a obtenção deve priorizar fragmentos raros;
* configurar estratégias como `headers-first`, `priority-first`, `rarest-first` ou políticas equivalentes;
* configurar pesos para seleção de fontes;
* configurar limites mínimos de vazão;
* configurar limites máximos de latência;
* configurar limites para circuit breaker de fontes;
* configurar revalidação de autorização durante transferências prolongadas.

Essas políticas pertencem ao domínio específico do Ponte Mesh e não devem ser forçadas dentro da API S3-like quando não houver correspondência natural com o modelo S3.

## Segurança de configuração

A configuração deve seguir princípios de segurança desde o início do projeto.

Regras obrigatórias:

* segredos não devem ser versionados;
* credenciais administrativas não devem ser reutilizadas por réplicas;
* credenciais de réplica devem ser separadas de credenciais de usuários e aplicações;
* chaves de assinatura devem permitir rotação;
* revogação de credenciais deve ser prevista desde o início;
* configurações inseguras devem falhar fechadas, negando acesso;
* permissões devem ser explícitas;
* ausência de configuração obrigatória deve impedir inicialização segura;
* logs não devem expor segredos, tokens, chaves ou credenciais;
* configurações de desenvolvimento não devem ser aceitas automaticamente em produção.

## Configuração operacional

O papel operacional do processo deve ser explícito.

Valores conceituais possíveis:

* `origin`;
* `replica-edge`.

Não deve existir um modo separado para representar ausência de peers, ausência de réplicas ou execução sem malha distribuída. Essas situações fazem parte do comportamento normal do Origin.

Quando nenhuma fonte auxiliar estiver disponível, o Origin deve continuar atendendo diretamente às operações autorizadas, preservando o modelo de segurança, manifesto, integridade e auditoria.

## Síntese

A configuração do Ponte Mesh deve separar claramente os parâmetros do **Origin** e do **Replica/Edge**.

O **Origin** é sempre a autoridade central e deve continuar funcional mesmo sem peers ou réplicas.

O **Replica/Edge** é um componente auxiliar, autorizado pelo Origin, usado para reforçar disponibilidade e reduzir carga em cenários apropriados.

A distribuição híbrida deve ser controlada por políticas explícitas. Ela é uma otimização operacional, não uma condição obrigatória para o servidor cumprir sua finalidade.
