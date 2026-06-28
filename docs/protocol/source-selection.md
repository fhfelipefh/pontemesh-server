# Seleção de fontes

A seleção de fontes é executada pelo SDK com base no pacote de acesso, no manifesto, nas políticas e na lista de fontes autorizadas emitidas pelo Origin.

O SDK não deve escolher fontes de forma livre ou autônoma. Toda fonte utilizada na obtenção de um fragmento deve ter sido autorizada pelo Origin e deve estar dentro do escopo, da validade e das políticas aplicáveis.

A seleção de fontes é uma decisão operacional do plano de dados, mas permanece subordinada ao plano de controle centralizado no Origin.

## Objetivo

O objetivo da seleção de fontes é escolher, para cada fragmento, a fonte mais adequada entre as fontes autorizadas.

Essa escolha deve equilibrar:

* segurança;
* autorização;
* disponibilidade;
* desempenho;
* integridade;
* estabilidade;
* preservação de progresso;
* redução de carga no Origin;
* continuidade da obtenção.

O P2P e o Replica/Edge devem ser usados quando forem autorizados, saudáveis e tecnicamente vantajosos.

O Origin deve permanecer como fonte direta e fonte final de garantia.

## Fontes possíveis

As fontes conceituais são:

* `PEER`;
* `REPLICA_EDGE`;
* `ORIGIN`.

## `PEER`

Representa um cliente autorizado que pode compartilhar temporariamente fragmentos já obtidos.

Peers são fontes auxiliares e potencialmente instáveis.

Um peer não é autoridade sobre autorização, manifesto, hash, política ou disponibilidade final.

Todo fragmento recebido de peer deve ser validado por hash conforme o manifesto emitido ou validado pelo Origin.

## `REPLICA_EDGE`

Representa um nó servidor auxiliar, mais estável que peers comuns, autorizado pelo Origin para replicar e servir fragmentos.

Replica/Edge pode reforçar a disponibilidade do plano de dados, mas não substitui o Origin como autoridade.

A réplica deve estar autenticada, autorizada, saudável e dentro do escopo vigente.

## `ORIGIN`

Representa o servidor de origem e autoridade central da arquitetura.

O Origin é fonte direta e fonte final de garantia.

Quando não houver peers ou Replica/Edge elegíveis, o SDK deve recorrer ao Origin, preservando fragmentos já validados sempre que possível.

## Prioridade conceitual

A prioridade conceitual padrão é:

1. `PEER` autorizado, saudável e com o fragmento necessário.
2. `REPLICA_EDGE` autorizada, autenticada, saudável e com o fragmento necessário.
3. `ORIGIN` como fonte direta e fonte final de garantia.

Essa prioridade pode ser ajustada por política emitida pelo Origin.

Em alguns cenários, a política pode preferir Replica/Edge antes de peers, exigir Origin para objetos sensíveis, bloquear P2P ou priorizar fragmentos críticos diretamente pelo Origin.

## Atributos de fonte

Cada fonte candidata pode possuir os seguintes atributos conceituais:

* `sourceId`;
* `sourceType`;
* fragmentos disponíveis;
* expiração da autorização;
* escopo autorizado;
* vazão estimada;
* latência média;
* taxa de sucesso;
* falhas recentes;
* estado do circuito;
* data da última falha;
* data do último sucesso;
* prioridade retornada pela política;
* versão ou compatibilidade com o manifesto;
* estado de saúde, quando aplicável;
* restrições operacionais.

## `sourceId`

Identificador único da fonte dentro do contexto de obtenção.

Pode representar um peer, uma réplica ou o próprio Origin.

## `sourceType`

Tipo da fonte.

Valores conceituais:

* `PEER`;
* `REPLICA_EDGE`;
* `ORIGIN`.

## Fragmentos disponíveis

Lista ou representação dos fragmentos que a fonte informa possuir.

O SDK deve usar essa informação apenas como indicação operacional.

A posse declarada de um fragmento não elimina a validação de integridade após o recebimento.

## Expiração da autorização

Indica até quando a fonte pode ser utilizada.

Fontes com autorização expirada não devem ser usadas.

## Escopo autorizado

Define quais ações, objetos, versões ou fragmentos a fonte pode atender.

Uma fonte não deve ser usada fora do escopo autorizado pelo Origin.

## Vazão estimada

Estimativa de throughput da fonte.

Pode ser calculada com base em histórico recente, média móvel ou informações reportadas.

A vazão estimada ajuda a priorizar fontes com melhor desempenho.

## Latência média

Tempo médio de resposta observado para a fonte.

Fontes com latência elevada podem receber prioridade menor, especialmente para fragmentos críticos ou consumo progressivo.

## Taxa de sucesso

Proporção recente de solicitações bem-sucedidas feitas à fonte.

Fontes com alta taxa de sucesso tendem a ser preferidas dentro do mesmo tipo.

## Falhas recentes

Quantidade ou severidade de falhas observadas recentemente.

Falhas podem incluir:

* timeout;
* conexão recusada;
* fragmento inválido;
* resposta incompleta;
* baixa vazão;
* erro de autorização;
* erro de disponibilidade;
* divergência em relação ao manifesto.

## Estado do circuito

Estado usado pelo SDK para evitar insistência em fontes instáveis.

Valores conceituais:

* `CLOSED`;
* `OPEN`;
* `HALF_OPEN`.

## `CLOSED`

A fonte está disponível para uso normal.

## `OPEN`

A fonte apresentou falhas suficientes para ser temporariamente ignorada.

Fontes com circuito aberto não devem ser usadas enquanto estiverem nesse estado.

## `HALF_OPEN`

A fonte está em estado de teste.

O SDK pode realizar tentativas limitadas para verificar se a fonte voltou a responder adequadamente.

## Elegibilidade

Uma fonte só pode ser usada quando todas as condições obrigatórias forem satisfeitas:

* foi autorizada pelo Origin;
* a autorização não expirou;
* a fonte está dentro do escopo do pacote de acesso;
* a fonte informa possuir o fragmento solicitado;
* a fonte está acessível;
* a fonte não está com circuito aberto;
* a fonte não ultrapassou o limite de falhas;
* a fonte atende aos limites mínimos de latência e vazão;
* a fonte não foi revogada;
* o objeto não foi revogado;
* o pacote de acesso continua válido;
* a política permite uso daquele tipo de fonte;
* o fragmento solicitado pertence ao manifesto autorizado.

Se qualquer condição falhar, a fonte deve ser removida da seleção para aquele fragmento.

## Fontes não elegíveis

Uma fonte deve ser considerada não elegível quando:

* não foi autorizada pelo Origin;
* está fora do pacote de acesso;
* está expirada;
* foi revogada;
* está inacessível;
* não possui o fragmento;
* está com circuito aberto;
* enviou fragmentos inválidos repetidamente;
* excedeu limite de falhas;
* possui latência acima do limite;
* possui vazão abaixo do limite;
* não atende à política aplicável;
* pertence a uma versão incompatível do objeto;
* tenta servir conteúdo fora do escopo autorizado.

## Qualidade da fonte

Entre fontes elegíveis do mesmo tipo, o SDK pode ordenar por uma função de qualidade.

A função de qualidade pode considerar:

* vazão estimada;
* taxa de sucesso;
* latência média;
* penalidade por falhas;
* estado do circuito;
* disponibilidade do fragmento;
* estabilidade recente.

Exemplo conceitual:

```text id="r6l7d5"
Q(f) = (α * throughput) + (β * successRate) - (γ * latency) - (δ * failurePenalty)
```

Onde:

* `throughput` representa a vazão estimada da fonte;
* `successRate` representa a taxa recente de sucesso;
* `latency` representa a latência média observada;
* `failurePenalty` representa a penalidade por falhas recentes;
* `α`, `β`, `γ` e `δ` são pesos definidos pela política emitida pelo Origin.

Essa função não representa um novo protocolo P2P. Ela é uma heurística operacional para ordenar fontes autorizadas.

## Pesos definidos pelo Origin

Os pesos da função de qualidade devem vir da política emitida pelo Origin.

Isso permite ajustar o comportamento do SDK por:

* bucket;
* objeto;
* tipo de conteúdo;
* aplicação;
* usuário;
* região;
* política de segurança;
* objetivo de desempenho;
* necessidade de reduzir carga do Origin.

O SDK não deve fixar pesos críticos de forma incompatível com a política do Origin.

## Seleção por fragmento

A seleção deve ocorrer preferencialmente por fragmento.

Cada fragmento pode ter uma fonte diferente.

Exemplo:

* fragmento 1 vindo de peer;
* fragmento 2 vindo de Replica/Edge;
* fragmento 3 vindo do Origin por fallback;
* fragmento 4 vindo de outro peer.

Essa flexibilidade permite aproveitar fontes auxiliares sem comprometer a continuidade da transferência.

## Fluxo conceitual

Fluxo simplificado de seleção:

1. O SDK recebe pacote de acesso do Origin.
2. O SDK obtém ou interpreta o manifesto.
3. O SDK cria a fila de fragmentos pendentes.
4. Para cada fragmento, identifica fontes autorizadas.
5. Remove fontes expiradas, revogadas, inacessíveis ou fora de escopo.
6. Remove fontes que não possuem o fragmento.
7. Remove fontes com circuito aberto.
8. Aplica limites de latência, vazão e falhas.
9. Agrupa fontes por tipo.
10. Aplica prioridade conceitual ou política específica.
11. Ordena fontes elegíveis pela função de qualidade.
12. Solicita o fragmento à melhor fonte elegível.
13. Valida o fragmento recebido.
14. Atualiza métricas da fonte.
15. Em caso de falha, aplica troca de fonte ou fallback.

## Pseudocódigo conceitual

```text id="c55xmn"
para cada fragmento pendente:
    fontesElegiveis = filtrarFontesAutorizadas(fragmento)

    remover fontes expiradas
    remover fontes revogadas
    remover fontes sem o fragmento
    remover fontes inacessiveis
    remover fontes com circuito OPEN
    remover fontes fora dos limites de politica

    se existe PEER elegivel:
        fonte = melhorFontePorQualidade(PEER)
    senao se existe REPLICA_EDGE elegivel:
        fonte = melhorFontePorQualidade(REPLICA_EDGE)
    senao:
        fonte = ORIGIN

    resultado = baixarFragmento(fragmento, fonte)

    se resultado valido:
        marcar fragmento como VALIDATED
        atualizar metricas positivas da fonte
    senao:
        penalizar fonte
        recolocar fragmento na fila
        aplicar fallback se necessario
```

## Relação com fallback

A seleção de fontes e o fallback são mecanismos complementares.

A seleção escolhe a melhor fonte elegível.

O fallback decide o que fazer quando a fonte escolhida falha ou deixa de ser adequada.

O fallback pode:

* trocar para outro peer;
* trocar para Replica/Edge;
* obter o fragmento do Origin;
* migrar a sessão para o Origin;
* retornar ao plano distribuído quando novas fontes elegíveis surgirem.

## Relação com o pacote de acesso

O pacote de acesso define quais fontes podem ser usadas.

O SDK não deve usar fontes fora do pacote, mesmo que tecnicamente estejam disponíveis.

O pacote pode conter:

* lista de fontes autorizadas;
* validade das fontes;
* escopo das fontes;
* política de seleção;
* política de fallback;
* limites de tentativas;
* limites de timeout;
* regras de revalidação.

## Relação com o manifesto

O manifesto define quais fragmentos existem e como validá-los.

A seleção de fontes deve sempre considerar o manifesto autorizado.

Uma fonte só deve ser usada para um fragmento se:

* o fragmento existe no manifesto;
* a fonte informa possuir o fragmento;
* o fragmento está dentro do escopo autorizado;
* a versão do objeto é compatível.

## Relação com Replica/Edge

Replica/Edge deve ser considerada uma fonte auxiliar mais estável que peers comuns.

Ela pode receber prioridade quando:

* peers estão indisponíveis;
* peers apresentam falhas;
* peers possuem baixa vazão;
* o fragmento é crítico;
* a política exige maior previsibilidade;
* a malha P2P está com baixa densidade.

Replica/Edge deve ser ignorada quando:

* não está autorizada;
* está revogada;
* está expirada;
* não possui o fragmento;
* está indisponível;
* não está saudável;
* falhou repetidamente;
* está com circuito aberto.

## Relação com peers

Peers podem ser utilizados quando autorizados e vantajosos.

O SDK deve considerar que peers podem sofrer com:

* churn;
* NAT;
* firewall;
* baixa vazão;
* indisponibilidade;
* comportamento malicioso;
* fragmentos inválidos;
* desconexões.

Por isso, peers devem ser usados com validação, limites, circuit breaker e fallback.

## Relação com o Origin

O Origin deve ser usado quando:

* não há peers elegíveis;
* não há Replica/Edge elegível;
* o fragmento é crítico e a política exige Origin;
* fontes auxiliares falharam;
* fontes auxiliares estão expiradas;
* fontes auxiliares foram revogadas;
* o limite de falhas foi atingido;
* a sessão migrou para fallback total;
* a política bloqueia distribuição auxiliar.

O Origin deve continuar funcional mesmo sem peers ou Replica/Edge.

## Revalidação

Durante transferências longas, o SDK pode precisar revalidar a autorização junto ao Origin.

A revalidação pode atualizar:

* validade do pacote de acesso;
* fontes autorizadas;
* fontes revogadas;
* políticas aplicáveis;
* estado do objeto;
* endpoints de fallback;
* limites operacionais.

Se a revalidação indicar revogação ou expiração, o SDK deve remover fontes afetadas e aplicar a política do Origin.

## Métricas

A seleção de fontes deve gerar métricas para observabilidade.

Métricas recomendadas:

* fonte escolhida por fragmento;
* tipo da fonte escolhida;
* fontes elegíveis por fragmento;
* fontes rejeitadas por fragmento;
* motivo de rejeição;
* taxa de uso de peers;
* taxa de uso de Replica/Edge;
* taxa de uso do Origin;
* vazão média por fonte;
* latência média por fonte;
* taxa de sucesso por fonte;
* falhas por fonte;
* quantidade de circuitos abertos;
* fallback após escolha de fonte;
* bytes servidos por tipo de fonte;
* fragmentos válidos por fonte;
* fragmentos inválidos por fonte.

## Auditoria

Eventos de seleção de fontes podem ser auditados quando forem relevantes para segurança ou operação.

Eventos recomendados:

* fonte não autorizada rejeitada;
* fonte expirada rejeitada;
* fonte revogada rejeitada;
* fonte removida por circuito aberto;
* fonte removida por falhas repetidas;
* fonte enviou fragmento inválido;
* fallback acionado por fonte ruim;
* tentativa de uso de fonte fora do escopo;
* Replica/Edge removida das fontes elegíveis.

A auditoria não deve expor segredos, tokens, tickets sensíveis ou conteúdo dos objetos.

## Segurança

A seleção de fontes deve seguir regras de segurança rígidas.

Regras obrigatórias:

* usar apenas fontes autorizadas pelo Origin;
* não usar fonte expirada;
* não usar fonte revogada;
* não usar fonte fora do escopo;
* não aceitar fragmento sem validação;
* não confiar em hashes enviados por peers;
* não aceitar manifesto de peers ou Replica/Edge como autoridade;
* não permitir que Replica/Edge emita autorização própria;
* aplicar política fail-closed quando houver ambiguidade;
* remover fontes suspeitas ou instáveis;
* respeitar revalidação durante transferências longas.

A implementação deve usar bibliotecas consolidadas para autenticação, autorização, validação de tokens, assinatura, criptografia, hashing e comparação segura. Não devem ser criados mecanismos caseiros de segurança.

## Anti-abuso

A seleção de fontes deve considerar limites para evitar abuso.

Controles possíveis:

* limite de tentativas por fonte;
* limite de tentativas por fragmento;
* limite de fontes simultâneas;
* limite de fallback por sessão;
* limite de peers por objeto;
* limite de ranges contra o Origin;
* penalização de fontes inválidas;
* circuit breaker;
* rate limit para reportes;
* rejeição de fontes com comportamento suspeito.

Esses controles evitam que peers ruins, réplicas problemáticas ou clientes maliciosos gerem sobrecarga no Origin.

## Síntese

A seleção de fontes é executada pelo SDK, mas sempre com base nas regras emitidas pelo Origin.

O SDK deve escolher fontes autorizadas, saudáveis e adequadas para cada fragmento, priorizando peers e Replica/Edge quando forem vantajosos e recorrendo ao Origin como fonte final de garantia.

Nenhuma fonte fora do pacote de acesso deve ser utilizada.

Todo fragmento recebido deve ser validado conforme o manifesto autorizado.

A seleção de fontes é o mecanismo que permite aproveitar a distribuição híbrida sem abrir mão de controle, segurança e previsibilidade.
