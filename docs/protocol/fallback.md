# Fallback

Fallback é a troca automática para uma fonte mais confiável quando peers ou Replica/Edge não conseguem entregar fragmentos de forma adequada, segura ou vantajosa.

No Ponte Mesh, o fallback é parte essencial da arquitetura. Ele garante que a obtenção continue mesmo quando fontes auxiliares falham, expiram, ficam indisponíveis ou apresentam desempenho insuficiente.

O Origin permanece como fonte direta e fonte final de garantia.

## Objetivo

O objetivo do fallback é preservar a continuidade da obtenção sem abandonar o controle centralizado do Origin.

O fallback deve permitir que o SDK:

* troque uma fonte instável por outra fonte autorizada;
* preserve fragmentos já validados;
* evite reiniciar o objeto inteiro;
* recorra ao Origin quando fontes auxiliares não forem suficientes;
* continue respeitando autorização, manifesto, expiração, revogação e políticas aplicáveis.

Fallback não é bypass de segurança. Mesmo quando a transferência passa a ocorrer pelo Origin, a operação deve permanecer dentro do escopo autorizado.

## Regra central

O fallback deve preservar todos os fragmentos já validados.

Sempre que possível, o fallback deve atuar no nível do fragmento ou do intervalo de bytes, não no nível do arquivo inteiro.

Isso significa que uma falha em um peer, Replica/Edge ou fragmento específico não deve obrigar o SDK a reiniciar a obtenção completa do objeto.

Fragmentos marcados como `VALIDATED` não devem ser baixados novamente.

## Níveis de fallback

O fallback pode ocorrer em diferentes níveis.

### 1. Troca de peer

Quando um peer falha, o SDK pode tentar outro peer autorizado que possua o mesmo fragmento.

Esse nível mantém a distribuição por peers ativa.

### 2. Troca para Replica/Edge

Quando peers não forem suficientes, não estiverem disponíveis ou apresentarem muitas falhas, o SDK pode tentar obter o fragmento a partir de Replica/Edge autorizada.

Replica/Edge deve ser usada como fonte auxiliar mais estável que peers comuns, desde que esteja autorizada, saudável e com o fragmento necessário.

### 3. Fallback de fragmento para Origin

Quando peers e Replica/Edge falharem ou não forem elegíveis, o SDK deve obter o fragmento diretamente do Origin.

Esse é o fallback preferencial, pois preserva os demais fragmentos já obtidos de fontes auxiliares.

### 4. Fallback de intervalo de bytes para Origin

Quando o fragmento corresponder a um intervalo de bytes, o SDK pode solicitar ao Origin apenas o intervalo necessário.

Esse comportamento depende do suporte do Origin a requisições `Range`.

### 5. Fallback total da sessão

Quando várias fontes auxiliares falharem, muitos fragmentos apresentarem erro ou a política indicar risco operacional, o SDK pode migrar temporariamente toda a sessão de obtenção para o Origin.

Esse nível deve ser usado com cuidado, pois aumenta a carga no Origin.

### 6. Retorno ao plano distribuído

Quando novas fontes elegíveis surgirem ou fontes anteriores melhorarem, o SDK pode voltar a utilizar peers ou Replica/Edge, desde que a política permita.

Esse retorno deve respeitar autorização, expiração, disponibilidade e estado de circuito das fontes.

## Gatilhos de fallback

O fallback pode ser acionado por diferentes gatilhos.

Gatilhos comuns:

* timeout;
* baixa vazão;
* latência elevada;
* falhas repetidas;
* fragmento inválido;
* fragmento incompleto;
* fonte inacessível;
* circuito aberto;
* pacote de acesso expirado;
* fonte com autorização expirada;
* objeto revogado;
* pacote de acesso revogado;
* ausência de peers elegíveis;
* ausência de Replica/Edge elegível;
* fonte sem o fragmento solicitado;
* erro de autenticação;
* erro de autorização;
* resposta incompatível com o manifesto;
* política alterada durante a transferência;
* falha de revalidação em transferência longa.

## Estados de fragmento

O fallback deve operar sobre um mapa de progresso mantido pelo SDK.

Estados conceituais possíveis:

* `PENDING`;
* `DOWNLOADING`;
* `VALIDATED`;
* `FAILED`;
* `INVALID`;
* `FALLBACK`.

## Regras por estado

### `PENDING`

O fragmento ainda não foi obtido ou voltou para a fila após uma falha recuperável.

Pode ser solicitado a qualquer fonte autorizada e elegível.

### `DOWNLOADING`

O fragmento está em transferência.

Se a fonte falhar, exceder timeout ou perder autorização, o fragmento pode voltar para `PENDING`, `FAILED` ou `FALLBACK`, conforme a política.

### `VALIDATED`

O fragmento foi recebido e validado por hash.

Não deve ser baixado novamente.

Esse estado deve ser preservado durante fallback.

### `FAILED`

A tentativa de obtenção falhou por erro operacional, como timeout, fonte indisponível, conexão interrompida ou baixa vazão.

O SDK pode tentar outra fonte antes de recorrer ao Origin.

### `INVALID`

O fragmento foi recebido, mas falhou na validação de integridade, tamanho, intervalo ou manifesto.

Dados inválidos devem ser descartados.

A fonte que enviou o fragmento inválido deve receber penalização operacional.

### `FALLBACK`

O fragmento foi encaminhado para obtenção por fonte mais confiável, normalmente o Origin.

Esse estado deve ser usado quando o limite de falhas ou a política indicarem que continuar tentando fontes auxiliares não é adequado.

## Seleção de fonte durante fallback

Durante o fallback, o SDK deve selecionar fontes apenas entre aquelas autorizadas pelo Origin.

A ordem conceitual padrão é:

1. outro peer autorizado;
2. Replica/Edge autorizada;
3. Origin.

A seleção deve considerar:

* disponibilidade do fragmento;
* autorização da fonte;
* expiração da autorização;
* estado de revogação;
* vazão estimada;
* latência média;
* taxa de sucesso;
* falhas recentes;
* estado de circuito;
* política retornada pelo Origin.

## Circuit breaker

O SDK pode usar circuit breaker para evitar insistência em fontes instáveis.

Uma fonte pode ter seu circuito aberto quando apresentar:

* falhas repetidas;
* timeouts consecutivos;
* fragmentos inválidos;
* baixa vazão persistente;
* erros de autorização;
* inconsistência com manifesto;
* indisponibilidade recorrente.

Enquanto o circuito estiver aberto, a fonte deve ser ignorada temporariamente.

Após um intervalo ou mudança de estado, a fonte pode voltar a ser testada, conforme política.

## Fallback por fragmento

Fallback por fragmento é a estratégia preferencial.

Nesse modelo, apenas o fragmento problemático é obtido de outra fonte.

Exemplo:

1. SDK baixa fragmentos 1, 2 e 3 de peers.
2. Fragmentos 1 e 2 são validados.
3. Fragmento 3 falha.
4. SDK tenta outro peer ou Replica/Edge para o fragmento 3.
5. Após falhas repetidas, SDK baixa apenas o fragmento 3 do Origin.
6. Fragmentos 1 e 2 permanecem preservados.
7. O objeto é reconstruído apenas com fragmentos validados.

Esse comportamento reduz desperdício de banda e evita reinício completo da obtenção.

## Fallback por intervalo de bytes

Quando o Origin oferece suporte a `Range`, o SDK pode solicitar apenas o intervalo necessário.

Essa estratégia é importante para:

* retomada parcial;
* obtenção de fragmentos específicos;
* recuperação de partes críticas;
* preservação de progresso;
* redução de tráfego desnecessário.

Ranges inválidos, abusivos ou fora do escopo devem ser rejeitados pelo Origin.

## Fallback total da sessão

Fallback total da sessão ocorre quando o SDK decide obter todos os fragmentos restantes diretamente do Origin.

Esse comportamento pode ser acionado quando:

* muitos fragmentos falham;
* não há fontes auxiliares elegíveis;
* Replica/Edge está indisponível;
* peers apresentam baixa qualidade;
* o número de tentativas excedeu o limite;
* a política de segurança exige entrega direta;
* a revalidação indica mudança de política;
* a experiência do usuário está sendo prejudicada.

Mesmo nesse caso, fragmentos já validados devem ser preservados.

O fallback total não deve reiniciar o objeto inteiro se houver fragmentos válidos localmente.

## Retorno ao plano distribuído

O retorno ao plano distribuído pode ocorrer quando a política permitir e houver fontes elegíveis novamente.

Exemplos:

* novo peer autorizado aparece;
* Replica/Edge volta a ficar disponível;
* circuito de uma fonte passa para estado de teste;
* vazão de fonte auxiliar melhora;
* Origin retorna novas fontes autorizadas em revalidação.

O retorno ao plano distribuído deve respeitar:

* pacote de acesso vigente;
* autorização;
* expiração;
* revogação;
* política aplicável;
* integridade dos fragmentos;
* estado de circuito da fonte.

## Segurança

Fallback não deve reduzir segurança.

Regras obrigatórias:

* nenhuma fonte fora do pacote de acesso deve ser usada;
* fallback não deve ignorar expiração;
* fallback não deve ignorar revogação;
* fallback não deve aceitar fragmento sem validação;
* fallback não deve aceitar manifesto emitido por peer;
* fallback não deve permitir Replica/Edge atuar como autoridade;
* fallback não deve permitir obtenção fora do escopo autorizado;
* fallback para Origin ainda exige autorização válida;
* fragmentos inválidos devem ser descartados;
* fontes com comportamento suspeito devem ser penalizadas ou removidas temporariamente.

A implementação deve usar bibliotecas e frameworks consolidados para segurança, autenticação, autorização, assinatura, tokens, validação criptográfica e comparação segura.

## Relação com o Origin

O Origin é a fonte final de garantia.

Durante fallback, o Origin pode:

* entregar o objeto diretamente;
* entregar fragmentos específicos;
* atender requisições por intervalo de bytes;
* revalidar autorização;
* informar mudança de política;
* revogar pacote de acesso;
* atualizar lista de fontes autorizadas;
* registrar métricas de fallback;
* registrar eventos de auditoria.

O Origin deve manter suporte a `Range requests` para permitir fallback granular.

## Relação com Replica/Edge

Replica/Edge pode ser usada como nível intermediário de fallback.

Ela deve ser considerada quando:

* peers falham;
* peers não possuem o fragmento;
* peers estão atrás de restrições de rede;
* peers apresentam baixa vazão;
* a política prioriza uma fonte mais estável antes do Origin.

Replica/Edge deve ser ignorada quando:

* não estiver autorizada;
* estiver revogada;
* estiver expirada;
* não possuir o fragmento;
* apresentar falhas repetidas;
* estiver com circuito aberto;
* não validar autorização apresentada pelo SDK;
* não estiver saudável.

## Relação com peers

Peers são fontes auxiliares temporárias e potencialmente instáveis.

O fallback deve estar preparado para lidar com:

* churn;
* NAT;
* firewall;
* baixa disponibilidade;
* baixa vazão;
* fragmentos inválidos;
* desconexões;
* ausência de peers;
* peers maliciosos.

Falha de peer não deve comprometer a obtenção do objeto.

O SDK deve tentar outras fontes autorizadas e, se necessário, recorrer ao Origin.

## Revalidação durante transferências longas

Transferências longas podem exigir revalidação junto ao Origin.

A revalidação pode verificar:

* se o pacote de acesso continua válido;
* se o objeto continua disponível;
* se houve revogação;
* se houve alteração de política;
* se fontes autorizadas mudaram;
* se o fallback deve ser alterado;
* se a sessão deve migrar para Origin;
* se a transferência deve ser interrompida.

Caso a revalidação falhe, o SDK deve aplicar a política definida pelo Origin.

## Métricas de fallback

Fallback deve ser mensurável.

Métricas recomendadas:

* taxa de fallback;
* taxa de fallback por fragmento;
* taxa de fallback por sessão;
* quantidade de fragmentos obtidos por fallback;
* quantidade de bytes obtidos por fallback;
* tempo médio até fallback;
* motivo do fallback;
* fonte original;
* fonte final;
* fragmentos preservados;
* bytes preservados;
* bytes desperdiçados;
* tentativas antes do fallback;
* fallback de peer para peer;
* fallback de peer para Replica/Edge;
* fallback de peer para Origin;
* fallback de Replica/Edge para Origin;
* sessões migradas para Origin;
* sessões que retornaram ao plano distribuído.

Essas métricas ajudam a avaliar se a arquitetura está reduzindo carga do Origin sem comprometer disponibilidade.

## Auditoria

Eventos de fallback podem ser auditados quando tiverem relevância operacional ou de segurança.

Eventos recomendados:

* fallback acionado;
* fallback negado por política;
* fallback por fragmento;
* fallback total de sessão;
* fonte removida por falhas;
* fonte removida por circuito aberto;
* fonte removida por revogação;
* fonte removida por expiração;
* fragmento inválido recebido;
* revalidação exigida;
* revalidação negada;
* sessão migrada para Origin.

Auditoria não deve expor tokens, tickets, chaves, URLs temporárias completas ou conteúdo dos objetos.

## Anti-abuso

Fallback pode aumentar carga no Origin e deve possuir limites.

Controles recomendados:

* limite de tentativas por fragmento;
* limite de fallback por sessão;
* limite de concorrência contra o Origin;
* limite de ranges por sessão;
* limite de peers com falha antes de migrar;
* circuit breaker por fonte;
* rate limit por usuário, aplicação ou SDK;
* detecção de abuso por fallback excessivo;
* registro de padrões anormais.

O objetivo é evitar que fontes ruins, clientes maliciosos ou políticas inadequadas sobrecarreguem o Origin.

## Exemplo conceitual de fluxo

1. O SDK recebe pacote de acesso do Origin.
2. O SDK interpreta o manifesto.
3. O SDK seleciona um peer autorizado para um fragmento.
4. O peer não responde dentro do timeout.
5. O SDK marca a tentativa como falha.
6. O SDK tenta outro peer autorizado.
7. O segundo peer envia fragmento inválido.
8. O SDK descarta o fragmento e penaliza a fonte.
9. O SDK tenta Replica/Edge autorizada.
10. Replica/Edge está indisponível.
11. O SDK aciona fallback para o Origin apenas para aquele fragmento.
12. O Origin entrega o intervalo necessário.
13. O SDK valida o hash.
14. O fragmento é marcado como `VALIDATED`.
15. A obtenção continua com os demais fragmentos.

## Síntese

Fallback é o mecanismo que garante continuidade quando fontes auxiliares não conseguem entregar fragmentos de forma adequada.

Ele deve preservar progresso validado, operar preferencialmente por fragmento ou intervalo de bytes e recorrer ao Origin como fonte final de garantia.

Fallback não elimina autorização, manifesto, integridade, expiração ou revogação.

Ele é uma estratégia de robustez para permitir que o Ponte Mesh use distribuição híbrida quando vantajoso, sem depender dela para funcionar.
