# Métricas

Este documento define métricas conceituais para avaliar desempenho, disponibilidade, segurança, uso de fontes auxiliares e redução de carga no **Origin**.

As métricas devem permitir comparar o comportamento do Ponte Mesh em dois cenários principais:

1. **Cenário base cliente-servidor**, em que todo o conteúdo é servido diretamente pelo Origin.
2. **Cenário híbrido**, em que a obtenção pode combinar Origin, Replica/Edge e peers autorizados, com fallback automático para o Origin.

O objetivo das métricas não é apenas medir velocidade de download, mas também avaliar se a distribuição híbrida reduz a carga do Origin sem comprometer segurança, integridade, disponibilidade e previsibilidade operacional.

## Métrica principal do projeto

A métrica principal do projeto é a redução de carga no Origin.

```text
ReducaoOrigin = 1 - (BytesOriginHibrido / BytesOriginBase)
```

Onde:

* `BytesOriginBase`: quantidade de bytes servidos pelo Origin no cenário cliente-servidor tradicional.
* `BytesOriginHibrido`: quantidade de bytes servidos pelo Origin no cenário com distribuição híbrida, considerando P2P, Replica/Edge e fallback.
* valor próximo de `1` indica maior redução de carga no Origin;
* valor próximo de `0` indica pouco ganho com a distribuição híbrida;
* valor menor que `0` indica que o cenário híbrido consumiu mais bytes do Origin do que o cenário base.

Exemplo conceitual:

```text
BytesOriginBase = 100 GB
BytesOriginHibrido = 40 GB

ReducaoOrigin = 1 - (40 / 100)
ReducaoOrigin = 0,60
```

Nesse exemplo, a arquitetura híbrida reduziu em 60% a quantidade de bytes servidos diretamente pelo Origin.

## Métricas de tráfego por fonte

Essas métricas indicam de onde os dados foram efetivamente obtidos.

* bytes servidos pelo Origin;
* bytes servidos por Replica/Edge;
* bytes servidos por peers;
* percentual de tráfego servido pelo Origin;
* percentual de tráfego servido por Replica/Edge;
* percentual de tráfego servido por peers;
* quantidade de fragmentos servidos por fonte;
* quantidade de objetos entregues integralmente pelo Origin;
* quantidade de objetos entregues com participação de Replica/Edge;
* quantidade de objetos entregues com participação de peers;
* quantidade de objetos entregues com fallback parcial;
* quantidade de objetos entregues com fallback total.

Essas métricas ajudam a identificar se a distribuição híbrida está realmente reduzindo dependência do Origin ou se, na prática, a maior parte da entrega continua centralizada.

## Métricas de desempenho

As métricas de desempenho devem avaliar a experiência de obtenção do conteúdo.

* tempo total de download;
* tempo até primeiro uso;
* tempo até primeiro byte;
* tempo até primeiro fragmento validado;
* vazão média por cliente;
* vazão média por fonte;
* vazão média do Origin;
* vazão média de Replica/Edge;
* vazão média de peers;
* latência média por fonte;
* tempo médio de seleção de fonte;
* tempo médio de validação de fragmento;
* tempo médio de reconstrução do objeto;
* tempo médio de resposta da API S3-like;
* tempo médio de resposta da API Ponte Mesh;
* tempo médio para emissão de pacote de acesso;
* tempo médio para obtenção de manifesto.

## Métricas de consumo progressivo

Quando o objeto permitir uso durante a obtenção, como em cenários de streaming, visualização progressiva ou leitura parcial, devem ser observadas métricas específicas.

* tempo até início do uso;
* quantidade de fragmentos necessários para primeiro uso;
* atraso até fragmentos iniciais;
* falhas antes do primeiro uso;
* interrupções durante o consumo;
* tempo gasto aguardando fragmentos críticos;
* percentual de fragmentos iniciais obtidos do Origin;
* percentual de fragmentos iniciais obtidos de Replica/Edge;
* percentual de fragmentos iniciais obtidos de peers.

Essas métricas são úteis para avaliar políticas como `headers-first`, `priority-first`, fragmentos iniciais e priorização sequencial.

## Métricas de fragmentos

As métricas de fragmentos devem permitir avaliar granularidade, integridade e eficiência da obtenção.

* total de fragmentos por objeto;
* fragmentos baixados com sucesso;
* fragmentos pendentes;
* fragmentos em andamento;
* fragmentos validados;
* fragmentos inválidos;
* fragmentos descartados;
* fragmentos repetidos;
* tentativas por fragmento;
* média de tentativas por fragmento;
* fragmentos obtidos por fallback;
* fragmentos obtidos diretamente do Origin;
* fragmentos obtidos de Replica/Edge;
* fragmentos obtidos de peers;
* fragmentos invalidados por hash;
* fragmentos rejeitados por tamanho incorreto;
* fragmentos rejeitados por autorização expirada;
* fragmentos rejeitados por fonte não autorizada.

## Métricas de fallback

O fallback é uma parte essencial da arquitetura e deve ser medido separadamente.

* taxa de fallback;
* taxa de fallback por objeto;
* taxa de fallback por fragmento;
* quantidade de fallbacks para Origin;
* quantidade de fallbacks de peer para Replica/Edge;
* quantidade de fallbacks de peer para Origin;
* quantidade de fallbacks de Replica/Edge para Origin;
* tempo médio até acionar fallback;
* número médio de falhas antes do fallback;
* fragmentos preservados antes do fallback;
* bytes reaproveitados após fallback;
* bytes desperdiçados por falhas antes do fallback;
* sessões migradas totalmente para Origin;
* sessões que retornaram ao uso de fontes auxiliares após fallback.

Essas métricas devem demonstrar se o fallback está preservando progresso validado e evitando reinício desnecessário da obtenção completa do objeto.

## Métricas de Replica/Edge

As métricas de Replica/Edge devem avaliar disponibilidade, sincronização, segurança e contribuição para redução de carga no Origin.

* réplicas registradas;
* réplicas ativas;
* réplicas indisponíveis;
* objetos sincronizados;
* fragmentos sincronizados;
* atraso de sincronização;
* tempo médio de sincronização;
* bytes sincronizados a partir do Origin;
* bytes servidos por réplica;
* fragmentos servidos por réplica;
* objetos atendidos por réplica;
* falhas de autenticação;
* falhas de autorização;
* falhas de sincronização;
* revogações recebidas;
* revogações aplicadas;
* disponibilidade reportada;
* saúde reportada;
* espaço local utilizado;
* limite de armazenamento atingido;
* limite de banda atingido;
* divergências entre catálogo do Origin e disponibilidade anunciada pela réplica.

## Métricas de peers

As métricas de peers devem avaliar a utilidade real da colaboração temporária entre clientes.

* peers autorizados;
* peers ativos;
* peers indisponíveis;
* peers removidos por falha;
* peers removidos por expiração;
* peers removidos por revogação;
* bytes servidos por peers;
* fragmentos servidos por peers;
* taxa de sucesso por peer;
* taxa de falha por peer;
* latência média por peer;
* vazão média por peer;
* fragmentos inválidos recebidos de peers;
* peers bloqueados por envio inválido;
* peers ignorados por circuit breaker;
* tempo médio de permanência de peers disponíveis;
* densidade média de peers por objeto;
* quantidade média de fontes por fragmento.

Essas métricas ajudam a avaliar se o P2P está contribuindo de forma relevante ou se a malha de peers está instável, pequena ou pouco vantajosa.

## Métricas de seleção de fontes

A seleção de fontes deve ser observável para permitir ajustes nas políticas do SDK e do Origin.

* fonte escolhida por fragmento;
* motivo da escolha da fonte;
* fontes elegíveis por fragmento;
* fontes rejeitadas por fragmento;
* rejeições por autorização expirada;
* rejeições por ausência do fragmento;
* rejeições por baixa vazão;
* rejeições por alta latência;
* rejeições por falhas recentes;
* rejeições por circuit breaker aberto;
* alterações de fonte durante a transferência;
* distribuição de escolhas entre Origin, Replica/Edge e peers.

Essas métricas ajudam a explicar por que o SDK escolheu determinada fonte e por que fontes auxiliares foram ou não utilizadas.

## Métricas de circuit breaker

O circuit breaker evita insistência em fontes instáveis.

Métricas recomendadas:

* fontes com circuito aberto;
* fontes em estado de teste;
* fontes recuperadas após falha;
* tempo médio em circuito aberto;
* quantidade de falhas antes da abertura do circuito;
* quantidade de tentativas após reabertura;
* taxa de recuperação de fontes;
* fontes definitivamente removidas da sessão;
* impacto do circuit breaker na taxa de fallback.

## Métricas de disponibilidade

Essas métricas indicam se objetos, fragmentos e fontes estão disponíveis para obtenção.

* objetos disponíveis;
* objetos expirados;
* objetos revogados;
* objetos indisponíveis;
* objetos bloqueados;
* fragmentos disponíveis por objeto;
* fontes disponíveis por objeto;
* fontes disponíveis por fragmento;
* objetos sem fontes auxiliares;
* objetos com apenas Origin disponível;
* objetos com Replica/Edge disponível;
* objetos com peers disponíveis;
* tempo médio de indisponibilidade;
* falhas por ausência de fonte elegível.

A ausência de peers ou Replica/Edge não deve ser tratada como falha do servidor. Ela deve ser registrada como condição operacional em que a entrega ocorre diretamente pelo Origin.

## Métricas de segurança

As métricas de segurança devem identificar uso indevido, tentativas inválidas e violações de política.

* pacotes de acesso emitidos;
* pacotes de acesso negados;
* pacotes de acesso expirados;
* pacotes de acesso revogados;
* tentativas com credenciais expiradas;
* tentativas com credenciais inválidas;
* tentativas com escopo insuficiente;
* tentativas de acesso a objeto revogado;
* tentativas de acesso a objeto expirado;
* tentativas de acesso a fonte não autorizada;
* tentativas de replay bloqueadas;
* tentativas de uso de manifesto inválido;
* tentativas de uso de manifesto expirado;
* fragmentos rejeitados por falha de integridade;
* requisições negadas por política fail-closed;
* falhas de autenticação de Replica/Edge;
* falhas de autorização de Replica/Edge;
* peers bloqueados por comportamento inválido.

## Métricas de auditoria

A auditoria deve permitir rastrear eventos sensíveis e decisões relevantes.

Eventos recomendados:

* criação de bucket;
* remoção lógica de objeto;
* publicação de objeto;
* emissão de pacote de acesso;
* revogação de objeto;
* revogação de usuário;
* revogação de aplicação;
* revogação de réplica;
* alteração de política de bucket;
* alteração de política de objeto;
* alteração de política de fallback;
* alteração de política de Replica/Edge;
* registro de réplica;
* sincronização de réplica;
* falha de autenticação;
* falha de autorização;
* operação administrativa sensível;
* eventos MCP, quando existir;
* eventos de dashboard administrativo, quando existir.

A auditoria deve registrar quem executou a ação, quando ocorreu, qual recurso foi afetado e qual foi o resultado da operação.

## Métricas da API S3-like

A API S3-like deve possuir métricas próprias para avaliar compatibilidade, uso e desempenho.

* requisições de criação de bucket;
* requisições de listagem de buckets;
* requisições de envio de objeto;
* requisições de listagem de objetos;
* requisições `HEAD`;
* requisições `GET`;
* requisições `GET` com `Range`;
* requisições `DELETE`;
* URLs temporárias geradas;
* URLs temporárias expiradas;
* códigos de resposta por operação;
* latência por operação;
* tamanho médio dos objetos enviados;
* tamanho médio dos objetos recuperados;
* erros por autenticação;
* erros por autorização;
* erros por objeto inexistente;
* erros por range inválido.

## Métricas da API Ponte Mesh

A API Ponte Mesh deve medir os contratos específicos da arquitetura híbrida.

* pacotes de acesso solicitados;
* pacotes de acesso emitidos;
* pacotes de acesso negados;
* manifestos solicitados;
* manifestos emitidos;
* estados de disponibilidade consultados;
* fontes autorizadas retornadas;
* políticas retornadas ao SDK;
* revogações executadas;
* métricas consultadas;
* eventos de auditoria consultados;
* configurações alteradas;
* falhas por política inválida;
* falhas por ausência de permissão;
* falhas por recurso indisponível.

## Métricas para avaliação experimental

Para comparar cenários, recomenda-se registrar os mesmos objetos, clientes e condições em dois modos de avaliação:

### Cenário base

Todo o conteúdo é servido diretamente pelo Origin.

Métricas mínimas:

* bytes servidos pelo Origin;
* tempo total de download;
* vazão média por cliente;
* tempo até primeiro uso;
* falhas de transferência;
* uso de banda do Origin.

### Cenário híbrido

O conteúdo pode ser servido por Origin, Replica/Edge e peers autorizados.

Métricas mínimas:

* bytes servidos pelo Origin;
* bytes servidos por Replica/Edge;
* bytes servidos por peers;
* taxa de fallback;
* tempo total de download;
* tempo até primeiro uso;
* fragmentos invalidados;
* tentativas por fragmento;
* redução de carga no Origin;
* disponibilidade percebida pelo cliente.

## Indicadores derivados

Além das métricas brutas, o sistema pode calcular indicadores derivados.

### Percentual de tráfego do Origin

```text
PercentualOrigin = BytesOrigin / BytesTotaisEntregues
```

### Percentual de tráfego auxiliar

```text
PercentualAuxiliar = (BytesReplicaEdge + BytesPeers) / BytesTotaisEntregues
```

### Taxa de fallback por fragmento

```text
TaxaFallbackFragmento = FragmentosComFallback / TotalFragmentosSolicitados
```

### Taxa de invalidação por hash

```text
TaxaInvalidacaoHash = FragmentosInvalidosPorHash / TotalFragmentosRecebidos
```

### Taxa de sucesso de peers

```text
TaxaSucessoPeers = FragmentosValidosRecebidosDePeers / FragmentosSolicitadosAosPeers
```

### Taxa de contribuição de Replica/Edge

```text
TaxaContribuicaoReplicaEdge = BytesReplicaEdge / BytesTotaisEntregues
```

### Taxa de contribuição P2P

```text
TaxaContribuicaoP2P = BytesPeers / BytesTotaisEntregues
```

## Requisitos de coleta

A coleta de métricas deve respeitar os requisitos de segurança e privacidade do projeto.

Regras recomendadas:

* não registrar segredos, tokens, chaves ou credenciais;
* não registrar conteúdo dos objetos;
* registrar identificadores de forma controlada;
* permitir correlação por operação, objeto, bucket e sessão de transferência;
* separar métricas operacionais de eventos de auditoria;
* preservar rastreabilidade de ações administrativas;
* permitir exportação futura para sistemas de observabilidade;
* permitir agregação por bucket, objeto, aplicação, usuário, réplica e fonte.

## Síntese

As métricas do Ponte Mesh devem demonstrar se a arquitetura híbrida está cumprindo seu propósito: reduzir carga do Origin quando houver fontes auxiliares viáveis, sem comprometer segurança, integridade, disponibilidade e previsibilidade.

A métrica central é a **redução de carga no Origin**, mas ela deve ser analisada em conjunto com desempenho, fallback, integridade, segurança, disponibilidade, comportamento de Replica/Edge e contribuição real dos peers autorizados.
