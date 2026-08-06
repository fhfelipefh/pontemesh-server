# Operação em modo Replica/Edge

Este documento descreve a operação do `pontemesh-server` quando executado no papel de **Replica/Edge**.

O **Replica/Edge** é um nó auxiliar mais estável que peers comuns, utilizado para reforçar a disponibilidade do plano de dados e reduzir a carga do Origin em cenários nos quais a replicação for autorizada, segura e tecnicamente vantajosa.

Replica/Edge opera sob autoridade do **Origin**, que controla ingestão, catálogo, autenticação, autorização, manifestos, políticas, revogação, expiração e disponibilidade.

## Papel da Replica/Edge

Replica/Edge atua como fonte auxiliar de fragmentos ou objetos autorizados.

Seu papel é apoiar a distribuição de conteúdo no plano de dados, dentro das autorizações emitidas pelo Origin.

O Replica/Edge deve operar sempre dentro dos escopos, políticas e autorizações emitidos pelo Origin.

## Responsabilidades

Replica/Edge deve ser responsável por:

* autenticar-se com o Origin;
* operar com identidade própria;
* utilizar credencial, certificado ou chave própria;
* obter plano de sincronização autorizado;
* sincronizar apenas objetos ou fragmentos permitidos;
* validar integridade dos conteúdos sincronizados;
* armazenar localmente fragmentos ou objetos autorizados;
* anunciar disponibilidade ao Origin;
* servir apenas fragmentos autorizados;
* validar autorização apresentada por SDKs antes de servir fragmentos;
* aplicar revogações recebidas do Origin;
* respeitar expiração de políticas, pacotes e escopos;
* reportar métricas de transferência;
* reportar métricas de saúde;
* registrar falhas de autenticação;
* registrar falhas de autorização;
* registrar eventos relevantes para auditoria.

## Relação com o Origin

Toda operação relevante da Replica/Edge deve estar subordinada ao Origin.

O Origin deve decidir:

* quais réplicas são válidas;
* quais réplicas estão revogadas;
* quais buckets podem ser sincronizados;
* quais objetos podem ser replicados;
* quais fragmentos podem ser servidos;
* quais políticas estão vigentes;
* quais escopos a réplica possui;
* por quanto tempo a autorização é válida;
* quando a réplica deve parar de servir determinado conteúdo.

Replica/Edge opera com políticas, planos e credenciais vigentes.

## Autenticação com o Origin

Replica/Edge deve autenticar-se com o Origin antes de qualquer operação sensível.

Operações sensíveis incluem:

* consulta de plano de sincronização;
* download de objetos ou fragmentos;
* anúncio de disponibilidade;
* reporte de métricas;
* reporte de saúde;
* recebimento de revogações;
* recebimento de mudanças de política.

A autenticação deve usar mecanismos consolidados, como mTLS, assinatura forte de requisições, tokens curtos emitidos pelo Origin ou combinação desses mecanismos.

O projeto usa bibliotecas consolidadas para autenticação, assinatura, geração de tokens, criptografia e comparação segura.

## Autorização

Autenticação identifica a réplica. Autorização define cada operação permitida.

A autorização deve considerar:

* identidade da réplica;
* escopo da réplica;
* bucket;
* objeto;
* fragmento;
* operação solicitada;
* política aplicável;
* validade temporal;
* estado de revogação;
* estado de disponibilidade do objeto;
* plano de sincronização vigente.

Toda permissão deve ser explícita, limitada e revogável.

## Plano de sincronização

Replica/Edge deve obter do Origin um plano de sincronização autorizado.

O plano de sincronização pode conter:

* buckets permitidos;
* objetos permitidos;
* fragmentos permitidos;
* prioridade de sincronização;
* política de retenção local;
* validade do plano;
* limites de banda;
* limites de armazenamento;
* endpoints de origem;
* regras de expiração;
* revogações pendentes;
* mudanças de política;
* parâmetros de auditoria;
* parâmetros de métricas.

Replica/Edge não deve decidir autonomamente quais objetos ou fragmentos pode replicar.

## Liderança degradada do plano de dados

Quando houver múltiplas réplicas autorizadas para o mesmo objeto, o Origin pode
incluir no plano de sincronização um conjunto de eleição por objeto. Esse conjunto
contém as réplicas elegíveis, o líder determinístico e a época de eleição.

Essa liderança é restrita ao plano de dados. Ela não transforma Replica/Edge em
Origin, não permite emissão de novos pacotes de acesso, não altera políticas, não
administra usuários e não substitui o catálogo central.

Se a Replica/Edge perder temporariamente comunicação com o Origin, ela pode servir
conteúdo local em modo degradado somente quando todas as condições forem verdadeiras:

* o objeto já foi sincronizado e validado por hash;
* a réplica consta no último conjunto de eleição emitido pelo Origin;
* a réplica é o líder eleito para aquele objeto;
* o pacote de acesso e o token apresentados já foram revalidados antes pelo Origin;
* a autorização local ainda está dentro da janela curta de continuidade;
* não há revogação ou mudança de política já recebida para aquele conteúdo.

Respostas servidas nesse modo devem ser marcadas como degradadas. Quando o Origin
voltar, a réplica deve retornar ao fluxo normal de revalidação, aplicar políticas
pendentes e reportar métricas novamente.

Enquanto o Origin estiver indisponível, usuários e integrações podem manter a
obtenção de dados já autorizados, mas funcionalidades completas de controle,
administração, emissão de novas autorizações, alteração de políticas e ingestão
continuam dependendo do Origin.

## Sincronização

Durante a sincronização, Replica/Edge deve:

* validar se o plano de sincronização ainda está vigente;
* validar se possui escopo para o conteúdo solicitado;
* obter manifesto ou metadados de integridade autorizados;
* baixar apenas objetos ou fragmentos permitidos;
* validar tamanho, intervalo de bytes e hash;
* descartar dados inválidos ou incompletos;
* registrar falhas de sincronização;
* respeitar limites de banda e armazenamento;
* interromper sincronização caso receba revogação aplicável.

Conteúdo sincronizado pela Replica/Edge não deve ser considerado armazenamento primário.

O armazenamento primário continua sendo responsabilidade do Origin.

## Armazenamento local

Replica/Edge pode armazenar localmente objetos ou fragmentos autorizados.

O armazenamento local deve respeitar:

* escopo emitido pelo Origin;
* política de retenção;
* validade temporal;
* revogação;
* limite de armazenamento;
* isolamento de dados;
* integridade;
* versão do objeto;
* estado de disponibilidade.

A réplica deve deixar de anunciar e servir conteúdos que tenham sido revogados, expirados ou removidos da política aplicável.

## Anúncio de disponibilidade

Replica/Edge deve anunciar ao Origin quais objetos ou fragmentos possui disponíveis.

O anúncio de disponibilidade pode conter:

* identidade da réplica;
* bucket;
* objeto;
* versão;
* lista de fragmentos disponíveis;
* data da sincronização;
* validade da disponibilidade;
* estado de saúde;
* capacidade disponível;
* métricas resumidas.

O Origin pode usar essas informações para decidir se a réplica deve aparecer como fonte elegível para SDKs.

O Origin não deve tratar o anúncio como autoridade absoluta. O SDK ainda deve validar integridade dos fragmentos recebidos.

## Serviço de fragmentos para SDKs

Replica/Edge pode servir fragmentos para SDKs autorizados.

Antes de servir qualquer fragmento, a réplica deve validar:

* se a autorização foi emitida pelo Origin;
* se a autorização ainda está vigente;
* se a autorização permite o objeto solicitado;
* se a autorização permite o fragmento solicitado;
* se a própria réplica está autorizada como fonte;
* se o objeto não foi revogado;
* se o pacote de acesso não foi revogado;
* se a política permite serviço por Replica/Edge;
* se o fragmento existe localmente;
* se o fragmento local foi validado.

Replica/Edge deve negar a solicitação quando qualquer validação falhar.

## Restrições

Replica/Edge serve apenas conteúdo autorizado pelo Origin, com escopo explícito,
integridade validada e permissão vigente. Conteúdo local é armazenamento auxiliar.

## Revogação

Replica/Edge deve receber e aplicar revogações emitidas pelo Origin.

Revogações podem afetar:

* réplica;
* bucket;
* objeto;
* versão;
* fragmento;
* pacote de acesso;
* usuário;
* aplicação;
* política.

Ao receber uma revogação, Replica/Edge deve:

* interromper novas sincronizações afetadas;
* deixar de anunciar o conteúdo revogado;
* deixar de servir fragmentos afetados;
* atualizar o estado local;
* registrar evento de auditoria;
* reportar a aplicação da revogação ao Origin, quando exigido pela política.

A revogação não precisa prometer apagamento físico imediato em todos os casos, mas deve impedir novas operações autorizadas a partir da réplica.

## Métricas

Replica/Edge deve reportar métricas ao Origin.

Métricas recomendadas:

* bytes sincronizados a partir do Origin;
* bytes servidos para SDKs;
* objetos sincronizados;
* fragmentos sincronizados;
* fragmentos servidos;
* falhas de sincronização;
* falhas de autenticação;
* falhas de autorização;
* solicitações negadas;
* solicitações com autorização expirada;
* solicitações para objeto revogado;
* fragmentos inválidos detectados;
* revogações recebidas;
* revogações aplicadas;
* tempo médio de resposta;
* vazão média;
* taxa de erro;
* uso de armazenamento;
* disponibilidade reportada.

Essas métricas ajudam a avaliar a contribuição da Replica/Edge para redução de carga no Origin.

## Saúde operacional

Replica/Edge deve reportar periodicamente sua saúde ao Origin.

O reporte de saúde pode incluir:

* estado operacional;
* versão do software;
* tempo ativo;
* conectividade com o Origin;
* latência média com o Origin;
* uso de CPU;
* uso de memória;
* uso de armazenamento;
* espaço disponível;
* limite de banda;
* erros recentes;
* fila de sincronização;
* revogações pendentes;
* revogações aplicadas.

O Origin pode usar essas informações para incluir, remover ou reduzir prioridade da réplica como fonte elegível.

## Segurança

A comunicação entre Origin e Replica/Edge deve ser:

* autenticada;
* autorizada;
* protegida contra replay;
* auditada;
* revogável;
* restrita por escopo;
* compatível com expiração;
* compatível com rotação de credenciais.

Uma réplica comprometida deve poder ser removida das fontes elegíveis sem afetar o funcionamento do Origin.

Mesmo que uma réplica seja comprometida, ela não deve conseguir comprometer a integridade do objeto final, pois o SDK deve validar os fragmentos recebidos conforme manifesto autorizado pelo Origin.

## Proteção contra replay

Operações sensíveis devem possuir proteção contra replay.

Controles recomendados:

* timestamp com janela curta;
* nonce único por requisição sensível;
* assinatura incluindo método, caminho e hash do corpo;
* rejeição de requisições repetidas;
* expiração curta de tickets e pacotes;
* auditoria de tentativas rejeitadas.

Esses controles devem ser implementados com bibliotecas e mecanismos consolidados.

## Auditoria

Replica/Edge deve registrar eventos relevantes.

Eventos recomendados:

* autenticação com o Origin;
* falha de autenticação;
* falha de autorização;
* plano de sincronização recebido;
* sincronização iniciada;
* sincronização concluída;
* sincronização com falha;
* fragmento inválido detectado;
* anúncio de disponibilidade;
* solicitação de fragmento recebida;
* solicitação de fragmento negada;
* fragmento servido;
* revogação recebida;
* revogação aplicada;
* mudança de política recebida;
* métrica reportada;
* erro operacional relevante.

Logs não devem expor segredos, tokens completos, chaves privadas, tickets sensíveis ou URLs temporárias completas.

## Comportamento em falhas

Replica/Edge pode ficar indisponível, falhar, perder conectividade ou ser removida das fontes elegíveis.

Nesses casos:

* o SDK deve tentar outra fonte autorizada;
* o SDK pode recorrer ao Origin;
* o Origin deve registrar a falha ou indisponibilidade;
* a réplica pode ser temporariamente removida da lista de fontes elegíveis;
* falhas repetidas devem reduzir sua prioridade;
* revogação deve impedir uso futuro até nova autorização.

A falha da Replica/Edge aciona entrega pelo Origin como fonte de garantia.

## Síntese

Replica/Edge é um nó auxiliar mais estável que peers comuns, usado para reforçar o plano de dados.

Ele deve autenticar-se com o Origin, obter plano de sincronização autorizado, replicar apenas conteúdos permitidos, validar integridade, anunciar disponibilidade, servir fragmentos somente para SDKs autorizados, aplicar revogações e reportar métricas.

Replica/Edge é fonte auxiliar autorizada. A autoridade central permanece no Origin.
