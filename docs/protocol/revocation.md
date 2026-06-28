# Revogação, expiração e deleção lógica

Em uma arquitetura híbrida, revogação, expiração e deleção lógica não significam, necessariamente, apagamento físico instantâneo de todas as cópias transitórias já distribuídas.

O controle principal está em impedir novas obtenções autorizadas, bloquear novas emissões de pacotes de acesso, remover fontes revogadas das fontes elegíveis e interromper transferências longas quando a política exigir.

No Ponte Mesh, o **Origin** permanece como autoridade sobre disponibilidade, autorização, revogação, expiração e estado lógico dos objetos.

## Objetivo

Este documento define como o sistema deve tratar:

* revogação de objetos;
* expiração de objetos, pacotes de acesso, políticas e fontes;
* deleção lógica;
* bloqueio temporário;
* interrupção de transferências longas;
* remoção de fontes elegíveis;
* aplicação de revogações em Replica/Edge;
* registro de auditoria.

Esses mecanismos existem para preservar controle centralizado mesmo quando o plano de dados utiliza peers e Replica/Edge.

## Conceito central

Revogar ou remover logicamente um objeto não garante apagar imediatamente todas as cópias transitórias já existentes em peers, caches locais, SDKs ou Replica/Edge.

A regra central é:

* o Origin deixa de emitir novas autorizações;
* SDKs devem respeitar revalidações quando exigido;
* Replica/Edge deve parar de anunciar e servir conteúdo revogado;
* fontes revogadas devem ser removidas das fontes elegíveis;
* novas obtenções controladas devem ser negadas.

## Estados conceituais

Um objeto pode assumir diferentes estados de disponibilidade.

### `AVAILABLE`

O objeto está disponível.

Novas autorizações podem ser emitidas pelo Origin, desde que o solicitante esteja autenticado, autorizado e dentro das políticas aplicáveis.

### `EXPIRED`

O objeto, pacote, política ou autorização perdeu validade temporal.

Novas autorizações devem ser negadas, salvo se uma política administrativa renovar ou reativar explicitamente o recurso.

### `REVOKED`

O objeto, acesso, pacote, usuário, aplicação, réplica ou política foi revogado por decisão administrativa, regra de segurança ou mudança de política.

Novas autorizações devem ser bloqueadas.

### `DELETED_LOGICAL`

O objeto foi removido logicamente do catálogo ativo.

Esse estado impede novas obtenções autorizadas, mas não promete apagamento físico imediato de todas as cópias transitórias já distribuídas.

### `BLOCKED`

O objeto está temporariamente indisponível.

Esse estado pode ser usado para bloqueios administrativos, investigação de segurança, inconsistência de metadados, falha de integridade ou outra condição temporária.

## Revogação

Revogação é o bloqueio de novas autorizações de acesso ou operação.

A revogação pode afetar:

* objeto;
* bucket;
* versão;
* pacote de acesso;
* usuário;
* aplicação;
* SDK;
* Replica/Edge;
* peer;
* política;
* fonte autorizada.

## Expiração

Expiração é o fim da validade temporal de uma autorização, pacote, manifesto, URL temporária, política, fonte ou credencial.

Após a expiração, o recurso não deve continuar sendo aceito.

Exemplos de itens que podem expirar:

* pacote de acesso;
* URL temporária;
* ticket;
* token;
* manifesto;
* fonte autorizada;
* plano de sincronização de réplica;
* autorização de peer;
* política temporária;
* credencial operacional.

## Deleção lógica

Deleção lógica é a marcação de um objeto como removido ou indisponível no catálogo ativo do Origin.

Ela deve impedir novas autorizações e novas obtenções controladas.

A deleção lógica pode preservar registros necessários para:

* auditoria;
* histórico;
* consistência;
* rastreabilidade;
* métricas;
* recuperação administrativa;
* propagação de revogação;
* retenção conforme política.

A deleção lógica não deve ser confundida com apagamento físico imediato.

## Bloqueio temporário

O estado `BLOCKED` pode ser usado quando o objeto precisa ser impedido temporariamente de gerar novas autorizações.

Exemplos:

* suspeita de comprometimento;
* inconsistência de manifesto;
* falha de integridade;
* investigação administrativa;
* política em revisão;
* indisponibilidade temporária;
* necessidade de impedir distribuição por fontes auxiliares.

Um objeto bloqueado não deve gerar novos pacotes de acesso enquanto o bloqueio estiver vigente.

## Regras gerais

O sistema deve seguir as seguintes regras:

* o Origin deve parar de emitir pacotes de acesso para objeto revogado;
* o Origin deve parar de emitir pacotes de acesso para objeto expirado;
* o Origin deve parar de emitir pacotes de acesso para objeto removido logicamente;
* o Origin deve parar de emitir pacotes de acesso para objeto bloqueado;
* SDKs devem revalidar acessos longos quando a política exigir;
* Replica/Edge deve receber ou consultar revogações;
* Replica/Edge deve parar de anunciar fontes revogadas;
* Replica/Edge deve parar de servir fragmentos revogados;
* fontes revogadas devem ser removidas das fontes elegíveis;
* pacotes expirados ou revogados não devem ser aceitos;
* manifestos expirados ou revogados não devem autorizar novas transferências;
* uma réplica comprometida deve ser revogável independentemente dos objetos;
* auditoria deve registrar quem revogou, quando e qual escopo foi afetado.

## Responsabilidades do Origin

O Origin deve ser responsável por:

* manter o estado de disponibilidade dos objetos;
* decidir se novas autorizações podem ser emitidas;
* negar pacote de acesso para objetos indisponíveis;
* revogar objetos, usuários, aplicações, pacotes e réplicas;
* aplicar expiração de pacotes de acesso;
* aplicar expiração de fontes autorizadas;
* registrar deleção lógica;
* comunicar revogações a Replica/Edge quando aplicável;
* remover fontes revogadas das fontes elegíveis;
* impedir sincronização de conteúdo revogado;
* registrar eventos de auditoria;
* expor estado de disponibilidade para SDKs e APIs administrativas.

O Origin deve continuar sendo a fonte de verdade sobre o estado lógico do objeto.

## Responsabilidades do SDK

O SDK deve respeitar as regras emitidas pelo Origin.

Responsabilidades esperadas:

* verificar validade do pacote de acesso;
* verificar validade do manifesto;
* respeitar expiração;
* respeitar revogação;
* revalidar transferências longas quando a política exigir;
* interromper obtenção quando a revalidação indicar revogação;
* ignorar fontes revogadas ou expiradas;
* não usar peers fora do pacote de acesso;
* não usar Replica/Edge fora do pacote de acesso;
* não aceitar fragmentos de objeto revogado quando a política bloquear continuidade;
* reportar eventos relevantes de falha, expiração ou revogação.

## Responsabilidades da Replica/Edge

Replica/Edge deve operar dentro das políticas emitidas pelo Origin.

Responsabilidades esperadas:

* consultar revogações pendentes;
* receber mudanças de política;
* parar de anunciar conteúdo revogado;
* parar de servir conteúdo revogado;
* aplicar revogação de escopo;
* aplicar revogação da própria réplica;
* impedir serviço com pacote expirado;
* impedir serviço com pacote revogado;
* registrar tentativas inválidas;
* reportar aplicação de revogação ao Origin, quando exigido.

Replica/Edge não deve continuar servindo conteúdo com base em autorização antiga quando houver revogação aplicável.

## Revogação de Replica/Edge

Replica/Edge deve ser revogável independentemente dos objetos armazenados nela.

Uma réplica pode ser revogada por:

* comprometimento de credencial;
* comportamento suspeito;
* falhas repetidas;
* envio de dados inválidos;
* violação de política;
* expiração de certificado ou credencial;
* decisão administrativa;
* retirada operacional.

Após revogação:

* a réplica deve ser removida das fontes elegíveis;
* novos planos de sincronização não devem ser emitidos;
* anúncios de disponibilidade devem ser rejeitados;
* pacotes de acesso não devem listar a réplica como fonte;
* SDKs devem ignorar a réplica;
* eventos devem ser auditados.

A revogação de uma réplica não deve afetar a disponibilidade do Origin, pois o Origin continua sendo fonte final de garantia.

## Revogação de pacote de acesso

Um pacote de acesso pode ser revogado antes do fim de sua expiração.

Motivos possíveis:

* revogação do objeto;
* revogação do usuário;
* revogação da aplicação;
* alteração de política;
* abuso detectado;
* comprometimento de credencial;
* operação administrativa;
* erro de emissão;
* mudança de estado do objeto.

Após revogação, o pacote não deve ser aceito para novas operações.

Em transferências longas, o SDK deve revalidar conforme política e interromper a obtenção se a revogação for detectada.

## Revogação de objeto

Quando um objeto for revogado:

* o Origin deve negar novos pacotes de acesso;
* o Origin deve atualizar o estado do objeto para `REVOKED`;
* Replica/Edge deve parar de anunciar o objeto;
* Replica/Edge deve parar de servir fragmentos do objeto;
* SDKs devem interromper ou revalidar transferências conforme política;
* fontes relacionadas devem ser removidas da elegibilidade;
* eventos devem ser auditados.

A revogação de objeto não precisa apagar fisicamente cópias transitórias já distribuídas, mas deve impedir novas obtenções controladas.

## Expiração de objeto ou política

Quando a validade temporal de um objeto ou política expirar:

* novas autorizações devem ser negadas;
* pacotes de acesso antigos não devem ser renovados automaticamente;
* Replica/Edge deve deixar de anunciar conteúdo expirado quando aplicável;
* SDKs devem revalidar se a política exigir;
* o estado pode mudar para `EXPIRED`;
* auditoria deve registrar a transição quando relevante.

## Deleção lógica de objeto

Quando um objeto for removido logicamente:

* o estado deve ser alterado para `DELETED_LOGICAL`;
* o objeto deve sair do catálogo ativo de obtenção;
* novas autorizações devem ser negadas;
* operações de leitura devem respeitar a política de visibilidade;
* Replica/Edge deve deixar de anunciar e servir o objeto;
* eventos de auditoria devem ser registrados;
* metadados mínimos podem ser preservados para auditoria e consistência.

A deleção lógica não deve ser usada como promessa de apagamento físico imediato em fontes auxiliares ou peers.

## Estados e transições

Transições conceituais possíveis:

```text id="zmb3f6"
AVAILABLE -> EXPIRED
AVAILABLE -> REVOKED
AVAILABLE -> DELETED_LOGICAL
AVAILABLE -> BLOCKED
BLOCKED -> AVAILABLE
BLOCKED -> REVOKED
BLOCKED -> DELETED_LOGICAL
EXPIRED -> AVAILABLE, quando renovado por política administrativa
REVOKED -> AVAILABLE, apenas se política administrativa permitir reativação
```

A implementação final pode restringir algumas transições para simplificar segurança e auditoria.

## Revalidação em transferências longas

Transferências longas podem atravessar mudanças de política, revogação ou expiração.

O pacote de acesso pode definir:

* se a revalidação é obrigatória;
* intervalo de revalidação;
* endpoint de revalidação;
* comportamento em caso de falha;
* comportamento em caso de revogação;
* comportamento em caso de expiração;
* comportamento em caso de mudança de política.

Se a revalidação indicar revogação, expiração ou bloqueio, o SDK deve interromper a transferência ou aplicar o comportamento definido pelo Origin.

## Continuidade de transferências já iniciadas

A política deve definir o que acontece com transferências já iniciadas quando ocorre revogação ou expiração.

Opções conceituais:

* permitir conclusão da transferência já autorizada;
* exigir revalidação e interromper se revogado;
* interromper imediatamente;
* permitir apenas fragmentos já em andamento;
* impedir novas fontes auxiliares;
* migrar para Origin para encerramento controlado;
* bloquear completamente a sessão.

A decisão deve ser explícita na política.

## Cópias transitórias

Cópias transitórias podem existir em:

* peers;
* cache local do SDK;
* Replica/Edge;
* armazenamento temporário;
* buffers de transferência;
* fragmentos parcialmente baixados.

O Origin não deve prometer controle físico instantâneo sobre todas essas cópias.

O controle arquitetural está em:

* impedir novas autorizações;
* remover fontes elegíveis;
* ordenar que Replica/Edge pare de servir;
* exigir revalidação;
* impedir uso de pacotes expirados ou revogados;
* registrar auditoria.

## Segurança

Regras de segurança:

* revogação deve negar novas autorizações;
* expiração deve negar uso posterior;
* deleção lógica deve impedir novas obtenções;
* bloqueio deve impedir novas autorizações enquanto vigente;
* Replica/Edge revogada não deve servir conteúdo;
* pacote expirado não deve ser aceito;
* pacote revogado não deve ser aceito;
* manifesto expirado ou revogado não deve autorizar nova transferência;
* peers não devem ser confiáveis para informar estado de revogação;
* o Origin deve ser a fonte de verdade sobre estado de disponibilidade;
* configurações ambíguas devem falhar fechadas.

Mecanismos de token, assinatura, expiração e validação devem usar bibliotecas e frameworks consolidados. O projeto não deve implementar mecanismos próprios de segurança.

## Auditoria

Auditoria é obrigatória para ações sensíveis de revogação, expiração administrativa e deleção lógica.

Eventos recomendados:

* objeto revogado;
* objeto expirado;
* objeto bloqueado;
* objeto desbloqueado;
* objeto removido logicamente;
* pacote de acesso revogado;
* pacote de acesso expirado;
* réplica revogada;
* usuário revogado;
* aplicação revogada;
* política alterada;
* revalidação negada;
* tentativa de uso de pacote expirado;
* tentativa de uso de pacote revogado;
* tentativa de acesso a objeto revogado;
* tentativa de Replica/Edge servir conteúdo revogado.

Cada evento deve registrar:

* quem executou;
* quando executou;
* qual recurso foi afetado;
* qual escopo foi afetado;
* qual política foi aplicada;
* qual foi o resultado;
* identificador de correlação da operação.

Logs e auditoria não devem expor tokens completos, chaves privadas, tickets sensíveis, URLs temporárias completas ou conteúdo dos objetos.

## Métricas

Métricas recomendadas:

* objetos disponíveis;
* objetos expirados;
* objetos revogados;
* objetos removidos logicamente;
* objetos bloqueados;
* pacotes de acesso revogados;
* pacotes de acesso expirados;
* tentativas de uso de pacote expirado;
* tentativas de uso de pacote revogado;
* réplicas revogadas;
* fontes removidas por revogação;
* transferências interrompidas por revogação;
* transferências interrompidas por expiração;
* revalidações executadas;
* revalidações negadas;
* tempo médio entre revogação e aplicação em Replica/Edge.

## Relação com fallback

Revogação e expiração podem afetar fallback.

Regras:

* fallback não deve ignorar revogação;
* fallback não deve usar fonte expirada;
* fallback não deve usar fonte revogada;
* fallback para Origin ainda exige autorização válida;
* se o pacote de acesso for revogado, o SDK deve interromper ou revalidar conforme política;
* se apenas uma fonte auxiliar for revogada, o SDK pode tentar outra fonte autorizada;
* se o objeto for revogado, novas obtenções devem ser negadas.

## Relação com Replica/Edge

Replica/Edge deve manter sincronização com o Origin sobre estados de revogação, expiração e deleção lógica.

Pode fazer isso por:

* consulta periódica;
* recebimento de eventos;
* plano de sincronização atualizado;
* endpoint de mudanças de política;
* mecanismo seguro de notificação.

Independentemente do mecanismo, a réplica deve deixar de anunciar e servir conteúdo afetado pela revogação.

## Síntese

Revogação, expiração e deleção lógica são mecanismos de controle do plano de controle centralizado no Origin.

Eles não prometem apagar instantaneamente todas as cópias transitórias já distribuídas, mas impedem novas autorizações, removem fontes elegíveis e permitem interromper transferências longas quando a política exigir.

O Origin continua sendo a fonte de verdade sobre disponibilidade, revogação, expiração e estado lógico dos objetos.
