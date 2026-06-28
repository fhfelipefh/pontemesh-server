# Fragmentos

Objetos são transferidos como fragmentos para permitir paralelismo, retomada parcial, fallback granular, validação independente e melhor aproveitamento de fontes auxiliares.

No Ponte Mesh, um objeto não precisa ser obtido como uma unidade indivisível. Ele pode ser dividido logicamente em partes menores, permitindo que diferentes fragmentos sejam baixados de fontes distintas, como Origin, Replica/Edge ou peers autorizados.

Essa estratégia permite reduzir desperdício de banda, preservar progresso validado e evitar que uma falha localizada obrigue o SDK a reiniciar a obtenção completa do objeto.

## Objetivo

A fragmentação tem como objetivos principais:

* permitir obtenção paralela de partes do objeto;
* permitir validação independente por fragmento;
* permitir fallback por fragmento ou intervalo de bytes;
* preservar fragmentos já validados;
* reduzir dependência de uma única fonte;
* permitir retomada parcial;
* permitir uso progressivo do conteúdo quando aplicável;
* facilitar obtenção híbrida entre Origin, Replica/Edge e peers autorizados.

## Relação com o manifesto

Todo fragmento deve ser descrito pelo manifesto autorizado do objeto.

O manifesto deve conter, no mínimo:

* identificação do objeto;
* versão do objeto;
* lista de fragmentos;
* índice de cada fragmento;
* intervalo de bytes correspondente;
* tamanho esperado;
* hash de integridade;
* metadados necessários para reconstrução;
* política aplicável;
* informações de disponibilidade quando necessário.

O SDK deve usar o manifesto como fonte de verdade para validar os fragmentos recebidos.

Peers e Replica/Edge não devem ser autoridade sobre hashes, tamanhos, intervalos ou estrutura do objeto.

## Estrutura conceitual de um fragmento

Um fragmento pode ser representado conceitualmente pelos seguintes campos:

```text id="ai11k6"
fragmentId
objectId
objectVersion
index
byteRangeStart
byteRangeEnd
expectedSize
expectedHash
state
source
attempts
lastFailureReason
validatedAt
```

Esses campos não definem uma estrutura final obrigatória de implementação, mas indicam as informações mínimas necessárias para controle, validação, fallback e auditoria.

## Estados no SDK

O SDK deve manter um mapa local de progresso para acompanhar o estado de cada fragmento.

Estados conceituais:

* `PENDING`: ainda não solicitado.
* `DOWNLOADING`: em transferência.
* `VALIDATED`: recebido e validado por hash.
* `FAILED`: falha de transferência.
* `INVALID`: recebido, mas rejeitado por integridade, tamanho ou intervalo.
* `FALLBACK`: encaminhado para obtenção por fonte mais confiável, normalmente o Origin.

## `PENDING`

Estado inicial de um fragmento.

Indica que o fragmento ainda não foi solicitado ou voltou para a fila após uma falha recuperável.

Um fragmento `PENDING` pode ser solicitado a uma fonte autorizada e elegível, respeitando política, disponibilidade e pacote de acesso.

## `DOWNLOADING`

Indica que o fragmento está em transferência.

Durante esse estado, o SDK deve controlar:

* fonte usada;
* tempo de início;
* timeout;
* progresso parcial;
* tentativa atual;
* autorização vigente;
* possibilidade de retomada parcial.

Se a transferência falhar, o fragmento deve mudar para `FAILED`, `INVALID`, `PENDING` ou `FALLBACK`, conforme a causa e a política aplicável.

## `VALIDATED`

Indica que o fragmento foi recebido completamente e validado por hash.

Somente fragmentos `VALIDATED` podem ser usados na reconstrução lógica do objeto.

Fragmentos nesse estado não devem ser baixados novamente, mesmo que ocorra fallback em outros fragmentos.

## `FAILED`

Indica falha operacional na obtenção.

Exemplos:

* timeout;
* conexão interrompida;
* fonte inacessível;
* baixa vazão;
* erro de rede;
* fonte sem o fragmento;
* falha temporária da Replica/Edge;
* peer indisponível.

Um fragmento `FAILED` pode ser recolocado na fila ou encaminhado para fallback, dependendo do número de tentativas e da política definida pelo Origin.

## `INVALID`

Indica que o fragmento foi recebido, mas rejeitado.

Motivos possíveis:

* hash divergente;
* tamanho incorreto;
* intervalo de bytes incorreto;
* conteúdo incompleto;
* fragmento incompatível com o manifesto;
* fragmento de outro objeto ou versão;
* dados adulterados ou corrompidos.

Fragmentos inválidos devem ser descartados.

A fonte que enviou fragmento inválido deve ser penalizada e pode ser ignorada temporariamente pelo circuit breaker.

## `FALLBACK`

Indica que o fragmento foi encaminhado para obtenção por uma fonte mais confiável.

Normalmente, isso significa obter o fragmento ou intervalo diretamente do Origin.

Esse estado deve ser usado quando:

* o fragmento falhou muitas vezes;
* as fontes auxiliares não são elegíveis;
* a política exige recuperação pelo Origin;
* a fonte anterior foi revogada;
* a autorização da fonte expirou;
* o fragmento é crítico para continuidade;
* o SDK detectou baixa qualidade nas fontes disponíveis.

## Regras de integridade

A integridade dos fragmentos é obrigatória.

Regras principais:

* dados parciais não são fragmentos válidos;
* fragmentos devem ser validados por hash antes de serem aceitos;
* fragmentos inválidos devem ser descartados;
* fragmentos incompletos não devem ser marcados como concluídos;
* fragmentos recebidos de peers também devem ser validados;
* fragmentos recebidos de Replica/Edge também devem ser validados;
* fragmentos recebidos do Origin também podem ser validados, conforme política;
* hashes enviados por peers não devem ser aceitos como autoridade;
* o manifesto autorizado deve ser a referência para validação;
* fragmentos já validados não devem ser baixados novamente.

A origem do fragmento não elimina a necessidade de validação.

## Dados parciais

Dados parciais não devem ser tratados como fragmentos válidos.

Eles podem ser mantidos apenas como dados temporários não confiáveis quando:

* a fonte suporta retomada parcial;
* o protocolo permite continuação segura;
* o SDK consegue associar o parcial ao fragmento correto;
* o conteúdo parcial ainda será validado antes de ser aceito;
* a política permite esse comportamento.

Se não houver suporte seguro à retomada parcial, dados parciais devem ser descartados.

## Preservação de progresso

A perda de progresso deve ficar limitada ao fragmento em andamento que ainda não foi validado.

Exemplo:

1. O objeto possui 100 fragmentos.
2. O SDK já validou 60 fragmentos.
3. A fonte atual falha no fragmento 61.
4. Os 60 fragmentos validados devem ser preservados.
5. Apenas o fragmento 61 deve voltar para fila ou fallback.
6. O objeto não deve ser reiniciado do zero.

Essa regra é fundamental para a eficiência da arquitetura.

## Recuperação parcial

O Origin deve suportar recuperação por intervalo de bytes.

Esse suporte permite que o SDK obtenha apenas os fragmentos ou intervalos pendentes, sem reiniciar o objeto completo.

A recuperação parcial é necessária para:

* fallback granular;
* retomada de download;
* obtenção de fragmentos específicos;
* preservação de progresso validado;
* redução de tráfego desnecessário;
* alternância entre fontes;
* recuperação de fragmentos críticos.

## Range requests

O Origin deve preservar suporte a requisições `Range`.

Exemplo conceitual:

```http id="fw3hsw"
GET /{bucket}/{objectKey}
Range: bytes=1048576-2097151
```

Esse recurso permite que o SDK solicite exatamente o intervalo necessário para recuperar um fragmento.

Ranges inválidos, abusivos ou fora do escopo autorizado devem ser rejeitados.

## Fallback por fragmento

O fallback deve ocorrer preferencialmente no nível do fragmento.

Quando um fragmento falha em uma fonte, o SDK pode:

1. tentar outro peer autorizado;
2. tentar Replica/Edge autorizada;
3. obter o fragmento diretamente do Origin;
4. preservar todos os demais fragmentos já validados.

Esse comportamento evita reiniciar o objeto completo e reduz desperdício de banda.

## Fragmentos críticos

Alguns fragmentos podem ser considerados mais importantes dependendo do tipo de conteúdo.

Exemplos:

* fragmentos iniciais;
* cabeçalhos;
* índices;
* metadados internos;
* fragmentos próximos ao ponto atual de leitura;
* fragmentos necessários para primeiro uso;
* fragmentos raros na malha distribuída.

A política de obtenção pode priorizar esses fragmentos usando estratégias como:

* `headers-first`;
* `priority-first`;
* `rarest-first`;
* priorização sequencial;
* priorização por janela de continuidade.

## Fragmentos raros

Fragmentos raros são aqueles com baixa disponibilidade entre fontes autorizadas.

Priorizar fragmentos raros pode aumentar a redundância do conteúdo e reduzir risco de indisponibilidade futura.

Essa estratégia deve ser usada conforme política do Origin e contexto de obtenção.

## Endgame

Quando restarem poucos fragmentos ou quando alguns fragmentos apresentarem falhas repetidas, o SDK pode aplicar uma estratégia semelhante a endgame.

Nesse caso, o mesmo fragmento pode ser solicitado a mais de uma fonte autorizada, aceitando a primeira resposta válida.

Regras:

* apenas fontes autorizadas podem ser usadas;
* o primeiro fragmento válido deve ser aceito;
* respostas duplicadas devem ser descartadas;
* fragmentos devem ser validados por hash;
* a estratégia deve respeitar limites para evitar desperdício excessivo.

## Seleção de fontes por fragmento

Cada fragmento pode ser obtido de uma fonte diferente.

Fontes possíveis:

* Origin;
* Replica/Edge;
* peer autorizado.

O SDK deve escolher a fonte com base em:

* autorização;
* disponibilidade do fragmento;
* expiração;
* revogação;
* latência;
* vazão;
* taxa de sucesso;
* falhas recentes;
* estado de circuito;
* política de seleção;
* criticidade do fragmento.

## Relação com o Origin

O Origin é responsável por:

* armazenar o objeto primário;
* gerar ou disponibilizar manifesto;
* fornecer hashes esperados;
* autorizar obtenção;
* emitir pacote de acesso;
* servir fragmentos ou intervalos quando necessário;
* atuar como fonte final de garantia;
* preservar suporte a `Range`;
* registrar métricas de bytes servidos;
* aplicar revogação e expiração.

O Origin não deve depender de peers ou Replica/Edge para cumprir sua função.

## Relação com Replica/Edge

Replica/Edge pode armazenar e servir fragmentos autorizados.

Regras:

* só pode sincronizar fragmentos autorizados pelo Origin;
* deve validar integridade do que sincroniza;
* deve anunciar disponibilidade ao Origin;
* só deve servir fragmentos quando o SDK apresentar autorização válida;
* deve deixar de servir fragmentos revogados ou expirados;
* não deve emitir manifesto como autoridade;
* não deve emitir autorização própria.

## Relação com peers

Peers podem compartilhar temporariamente fragmentos já obtidos, quando a política permitir.

Regras:

* peer não é fonte confiável por padrão;
* peer não emite autorização;
* peer não define hash;
* peer não define manifesto;
* peer não decide política;
* fragmentos vindos de peer devem ser validados;
* peer que envia fragmento inválido deve ser penalizado;
* peer com falhas repetidas pode ser ignorado pelo circuit breaker.

## Auditoria e métricas

O sistema deve registrar métricas e eventos relacionados a fragmentos.

Métricas recomendadas:

* total de fragmentos por objeto;
* fragmentos solicitados;
* fragmentos baixados;
* fragmentos validados;
* fragmentos inválidos;
* fragmentos em fallback;
* tentativas por fragmento;
* bytes por fragmento;
* bytes por fonte;
* tempo médio por fragmento;
* fragmentos obtidos do Origin;
* fragmentos obtidos de Replica/Edge;
* fragmentos obtidos de peers;
* fragmentos descartados;
* falhas de integridade;
* falhas de timeout;
* falhas por fonte inacessível.

Eventos auditáveis:

* fragmento inválido recebido;
* fallback acionado para fragmento;
* fonte removida por falhas;
* fonte removida por circuito aberto;
* tentativa de obter fragmento sem autorização;
* tentativa de servir fragmento revogado;
* divergência entre manifesto e fragmento recebido.

## Segurança

A fragmentação não deve reduzir segurança.

Regras obrigatórias:

* nenhum fragmento deve ser aceito sem validação;
* nenhuma fonte deve ser usada fora do pacote de acesso;
* nenhum fragmento deve ser obtido fora do escopo autorizado;
* dados inválidos devem ser descartados;
* fontes suspeitas devem ser penalizadas;
* fragmentos de objetos revogados não devem ser servidos;
* fragmentos de versões incompatíveis devem ser rejeitados;
* logs não devem expor conteúdo dos fragmentos;
* hashes e validações devem usar bibliotecas confiáveis da plataforma.

O projeto não deve implementar mecanismos caseiros de criptografia, hashing seguro, assinatura ou validação de tokens.

## Exemplo conceitual

Fluxo simplificado:

1. O SDK recebe pacote de acesso do Origin.
2. O SDK obtém o manifesto autorizado.
3. O manifesto informa que o objeto possui 10 fragmentos.
4. O SDK cria um mapa local com os 10 fragmentos em estado `PENDING`.
5. O SDK solicita fragmentos a peers, Replica/Edge ou Origin.
6. Cada fragmento recebido é validado por hash.
7. Fragmentos válidos são marcados como `VALIDATED`.
8. Fragmentos inválidos são descartados e marcados como `INVALID`.
9. Fragmentos com falhas repetidas são marcados como `FALLBACK`.
10. O SDK obtém fragmentos problemáticos do Origin.
11. O objeto é reconstruído apenas com fragmentos validados.

## Síntese

Fragmentos são a base da obtenção híbrida no Ponte Mesh.

Eles permitem paralelismo, validação independente, retomada parcial, fallback granular e preservação de progresso.

O SDK deve tratar cada fragmento como uma unidade verificável.

O Origin deve fornecer manifesto, autorização, hashes esperados e suporte a recuperação por intervalo de bytes.

Replica/Edge e peers podem contribuir como fontes auxiliares, mas nenhum fragmento deve ser aceito sem validação de integridade.
