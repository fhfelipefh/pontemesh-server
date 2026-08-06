# Pacote de acesso

O **pacote de acesso** é emitido pelo Origin após autenticação, autorização e avaliação das políticas aplicáveis ao objeto, bucket, usuário, aplicação, SDK e contexto da requisição.

Ele representa uma autorização temporária para uma operação de obtenção controlada.

O pacote de acesso não deve ser tratado como permissão permanente. Ele deve possuir escopo definido, prazo de validade, fontes autorizadas e regras claras de uso.

## Objetivo

O pacote de acesso permite que o SDK obtenha um objeto de forma controlada, podendo utilizar Origin, Replica/Edge ou peers autorizados conforme política aplicável.

Ele deve fornecer ao SDK as informações necessárias para:

* identificar o objeto solicitado;
* acessar ou consultar o manifesto autorizado;
* conhecer os fragmentos e suas regras de validação;
* saber quais fontes podem ser utilizadas;
* aplicar políticas de seleção de fontes;
* aplicar políticas de fallback;
* respeitar expiração e revogação;
* revalidar acesso em transferências prolongadas;
* reportar métricas e falhas quando aplicável.

Nenhuma fonte fora do pacote de acesso deve ser considerada autorizada.

## Emissão

O pacote de acesso deve ser emitido exclusivamente pelo Origin.

Antes de emitir o pacote, o Origin deve validar:

* identidade do solicitante;
* autenticação;
* autorização;
* escopo solicitado;
* bucket;
* objeto;
* estado de disponibilidade;
* política de bucket;
* política de objeto;
* revogação;
* expiração;
* elegibilidade para uso de Replica/Edge;
* elegibilidade para uso de peers;
* limites operacionais aplicáveis.

Se qualquer validação falhar, o Origin deve negar a emissão do pacote.

## Conteúdo conceitual

Um pacote de acesso pode conter:

* identificação do pacote;
* identificação do objeto;
* bucket;
* chave do objeto;
* versão do objeto;
* manifesto ou referência segura ao manifesto;
* ticket, token, URL temporária ou credencial transitória;
* prazo de expiração;
* escopo de acesso;
* identidade ou referência do solicitante;
* aplicação solicitante, quando aplicável;
* fontes autorizadas;
* endpoint de fallback;
* política de seleção de fontes;
* política de seleção de fragmentos;
* limites de tentativas;
* limites de timeout;
* limites de fallback;
* instrução de revalidação durante transferências longas;
* permissões para compartilhamento temporário de fragmentos, quando permitido;
* parâmetros para reporte de métricas;
* identificador de correlação para auditoria.

## Escopo

O pacote de acesso deve possuir escopo explícito.

O escopo pode limitar:

* ação permitida;
* bucket;
* objeto;
* versão;
* fragmentos;
* usuário;
* aplicação;
* SDK;
* fontes autorizadas;
* duração da autorização;
* estratégia de obtenção;
* política de fallback;
* possibilidade de compartilhamento temporário;
* limites de uso.

O pacote não deve conceder permissão implícita a objetos, fragmentos, fontes ou ações fora do escopo autorizado.

## Fontes autorizadas

O pacote de acesso pode listar as fontes permitidas para a obtenção.

Tipos possíveis:

* `ORIGIN`;
* `REPLICA_EDGE`;
* `PEER`.

Cada fonte autorizada deve conter informações suficientes para o SDK decidir se pode utilizá-la, como:

* identificador da fonte;
* tipo da fonte;
* endpoint ou referência de conexão;
* fragmentos disponíveis, quando aplicável;
* validade da autorização;
* escopo da fonte;
* prioridade conceitual;
* restrições de uso;
* estado ou metadados de disponibilidade;
* parâmetros de fallback.

O Origin deve permanecer como fonte direta ou fonte final de garantia.

Replica/Edge e peers são fontes auxiliares condicionadas à política, disponibilidade e autorização.

## Manifesto

O pacote de acesso pode conter o manifesto completo ou uma referência segura para consulta do manifesto.

O manifesto deve permitir que o SDK:

* identifique os fragmentos do objeto;
* conheça intervalos de bytes;
* conheça tamanhos esperados;
* valide hashes de integridade;
* preserve progresso validado;
* reconstrua logicamente o objeto;
* aplique fallback por fragmento ou intervalo.

O manifesto deve ser emitido, assinado ou validado pelo Origin.

Peers e Replica/Edge não devem ser autoridade sobre o manifesto.

## Políticas de obtenção

O pacote de acesso pode incluir políticas que orientam o comportamento do SDK durante a obtenção.

Exemplos de políticas:

* prioridade entre `PEER`, `REPLICA_EDGE` e `ORIGIN`;
* habilitação ou bloqueio de P2P;
* habilitação ou bloqueio de Replica/Edge;
* estratégia `headers-first`;
* estratégia `priority-first`;
* estratégia `rarest-first`;
* priorização de fragmentos iniciais;
* priorização de fragmentos críticos;
* limites de falha antes do fallback;
* timeout por fonte;
* timeout por fragmento;
* número máximo de tentativas;
* regras de circuit breaker;
* revalidação obrigatória após determinado tempo;
* migração total da sessão para Origin em caso de falhas generalizadas.

Essas políticas são próprias do Ponte Mesh e não precisam caber no modelo S3-like.

## Fallback

O pacote de acesso deve indicar como o SDK deve proceder quando uma fonte falhar.

O fallback pode ocorrer:

* de peer para outro peer;
* de peer para Replica/Edge;
* de peer para Origin;
* de Replica/Edge para Origin;
* por fragmento;
* por intervalo de bytes;
* pela sessão inteira, quando necessário.

O fallback deve preservar fragmentos já validados sempre que possível.

O pacote deve informar endpoints e limites necessários para que o SDK recorra ao Origin sem reiniciar desnecessariamente a obtenção completa do objeto.

## Revalidação

Transferências longas podem exigir revalidação.

O pacote de acesso pode indicar:

* se a revalidação é obrigatória;
* intervalo de revalidação;
* endpoint de revalidação;
* condições que exigem nova consulta ao Origin;
* comportamento esperado em caso de expiração;
* comportamento esperado em caso de revogação;
* comportamento esperado em caso de mudança de política.

Se a revalidação falhar, o SDK deve interromper a obtenção ou aplicar a política definida pelo Origin.

## Revogação

O pacote de acesso deve ser revogável pelo Origin.

A revogação pode ocorrer por:

* revogação do objeto;
* revogação do usuário;
* revogação da aplicação;
* revogação da réplica;
* alteração de política;
* expiração administrativa;
* detecção de abuso;
* comprometimento de credencial;
* operação administrativa sensível.

Após revogação, o pacote não deve ser aceito em novas operações.

Em transferências prolongadas, o SDK pode ser obrigado a revalidar o pacote e interromper a obtenção se a revogação for detectada.

## Segurança

O pacote de acesso é um artefato sensível.

Regras obrigatórias:

* deve ser não adivinhável;
* deve ter expiração curta;
* deve possuir escopo explícito;
* deve ser vinculado ao solicitante e ao contexto autorizado;
* deve ser revogável pelo Origin;
* deve ser resistente a replay;
* não deve conceder permissão implícita;
* não deve autorizar fontes fora da lista permitida;
* não deve expor segredos permanentes;
* não deve ser aceito após expiração;
* não deve ser aceito após revogação;
* deve ser validado antes do uso por Replica/Edge;
* deve ser validado pelo SDK antes de iniciar a obtenção.

A implementação deve usar bibliotecas e frameworks consolidados para geração de tokens, assinatura, validação criptográfica, comparação segura, expiração e proteção contra replay.

Não devem ser implementados mecanismos próprios de criptografia, assinatura, geração de tokens ou validação de segurança.

## Proteção contra replay

O pacote de acesso deve ser resistente a replay.

Controles possíveis:

* expiração curta;
* nonce;
* identificador único do pacote;
* associação ao solicitante;
* associação ao objeto;
* associação ao escopo;
* assinatura do conteúdo;
* validação de timestamp;
* rejeição de uso após revogação;
* revalidação em operações longas;
* auditoria de tentativas repetidas.

A escolha exata do mecanismo deve ser feita com base em bibliotecas e padrões consolidados.

## Auditoria

A emissão e o uso do pacote de acesso devem gerar eventos de auditoria quando aplicável.

Eventos recomendados:

* pacote emitido;
* pacote negado;
* pacote expirado;
* pacote revogado;
* tentativa de uso de pacote expirado;
* tentativa de uso de pacote revogado;
* tentativa de uso fora de escopo;
* tentativa de uso por fonte não autorizada;
* revalidação executada;
* revalidação negada;
* fallback acionado;
* fonte rejeitada por política.

A auditoria deve permitir rastrear:

* quem solicitou;
* quando solicitou;
* qual objeto foi solicitado;
* qual escopo foi concedido;
* quais fontes foram autorizadas;
* qual política foi aplicada;
* qual foi o resultado da operação.

Logs e auditoria não devem expor tokens completos, chaves privadas, tickets sensíveis ou URLs temporárias completas sem mascaramento adequado.

## Relação com Replica/Edge

Replica/Edge só deve servir fragmentos quando o solicitante apresentar autorização válida emitida pelo Origin.

Antes de servir um fragmento, Replica/Edge deve validar:

* se o pacote foi emitido pelo Origin;
* se o pacote ainda está vigente;
* se o pacote permite aquele objeto;
* se o pacote permite aquele fragmento;
* se a réplica está listada como fonte autorizada;
* se o objeto não foi revogado;
* se a política permite serviço por Replica/Edge.

Replica/Edge não deve aceitar pacote expirado, revogado, adulterado ou fora de escopo.

## Relação com o SDK

O SDK deve usar o pacote de acesso como base para a obtenção controlada.

O SDK deve:

* validar o pacote antes de iniciar a obtenção;
* respeitar escopo e expiração;
* usar apenas fontes autorizadas;
* obter ou interpretar o manifesto autorizado;
* validar fragmentos por hash;
* aplicar políticas de seleção de fontes;
* aplicar políticas de fallback;
* revalidar acesso quando exigido;
* reportar métricas e falhas conforme contrato.

O SDK não deve usar peers ou réplicas fora da autorização emitida pelo Origin.

## Síntese

O pacote de acesso é o contrato temporário que autoriza uma obtenção no Ponte Mesh.

Ele conecta autenticação, autorização, manifesto, fontes autorizadas, políticas de seleção, fallback, expiração, revogação e auditoria.

O pacote deve ser emitido somente pelo Origin, possuir escopo explícito, expirar rapidamente, ser revogável e não permitir uso de fontes fora da lista autorizada.

Ele é essencial para garantir que a distribuição híbrida continue subordinada ao controle central do Origin.
