# Replica API

Este documento descreve a API conceitual exposta ou consumida por um componente **Replica/Edge** no Ponte Mesh.

O Replica/Edge é uma fonte auxiliar do plano de dados. Seu objetivo é reforçar a disponibilidade de objetos e fragmentos autorizados, reduzindo carga do Origin quando houver política, autorização e benefício técnico.

Replica/Edge não é autoridade independente. A autoridade sobre publicação, autorização, disponibilidade, manifesto, revogação, expiração e políticas continua sendo sempre o **Origin**.

## Responsabilidades da Replica/Edge

Replica/Edge deve ser capaz de:

* autenticar-se com o Origin usando identidade própria;
* consultar planos de sincronização autorizados;
* baixar objetos ou fragmentos autorizados a partir do Origin;
* validar integridade dos dados sincronizados;
* armazenar localmente objetos ou fragmentos autorizados;
* anunciar disponibilidade de fragmentos ao Origin;
* servir fragmentos para SDKs autorizados;
* rejeitar solicitações sem autorização válida emitida pelo Origin;
* receber e aplicar revogações;
* receber mudanças de política;
* reportar saúde operacional;
* reportar métricas de sincronização e transferência;
* registrar falhas de autenticação, autorização, sincronização e serviço de fragmentos.

## Fluxos principais

### 1. Autenticação da Replica/Edge com o Origin

Antes de sincronizar ou anunciar qualquer conteúdo, a Replica/Edge deve autenticar-se com o Origin.

A autenticação deve usar mecanismos consolidados, como mTLS, assinatura forte de requisições, tokens curtos emitidos pelo Origin ou combinação desses mecanismos.

Não devem ser implementados algoritmos próprios de autenticação, assinatura, geração de tokens ou criptografia.

A autenticação deve identificar de forma inequívoca:

* identidade da réplica;
* credencial, certificado ou chave utilizada;
* escopos permitidos;
* validade da credencial;
* política aplicável;
* estado da réplica.

Uma réplica autenticada ainda precisa ser autorizada. Autenticação prova identidade, mas autorização define o que a réplica pode sincronizar, armazenar, anunciar e servir.

### 2. Consulta de plano de sincronização

Replica/Edge deve consultar o Origin para obter um plano de sincronização autorizado.

O plano de sincronização pode conter:

* buckets autorizados;
* objetos autorizados;
* fragmentos autorizados;
* prioridade de sincronização;
* política de retenção local;
* validade do plano;
* limites de banda;
* limites de armazenamento;
* regras de expiração;
* revogações pendentes;
* mudanças de política;
* endpoints de origem para sincronização;
* parâmetros de auditoria e métricas.

Replica/Edge não deve decidir de forma autônoma quais objetos pode replicar.

### 3. Sincronização de objeto ou fragmento

Após receber um plano válido, Replica/Edge pode baixar objetos ou fragmentos autorizados a partir do Origin.

Durante a sincronização, a réplica deve:

* validar se o plano ainda está vigente;
* validar se possui escopo para o bucket, objeto ou fragmento;
* obter manifesto autorizado ou informações equivalentes de integridade;
* baixar apenas conteúdo autorizado;
* validar hash, tamanho e intervalo de bytes dos fragmentos;
* descartar dados inválidos, incompletos ou incompatíveis;
* registrar falhas de sincronização;
* respeitar limites de banda e armazenamento;
* interromper sincronização se receber revogação aplicável.

### 4. Armazenamento local

Replica/Edge pode manter localmente objetos ou fragmentos autorizados.

O armazenamento local deve respeitar:

* escopo autorizado pelo Origin;
* política de retenção;
* expiração;
* revogação;
* limite de armazenamento;
* isolamento de dados;
* integridade do conteúdo;
* rastreabilidade de origem e versão.

Conteúdo armazenado na Replica/Edge não deve ser tratado como armazenamento primário. O armazenamento primário e a autoridade sobre o objeto continuam no Origin.

### 5. Anúncio de disponibilidade

Após sincronizar conteúdo autorizado, Replica/Edge deve anunciar ao Origin quais objetos ou fragmentos possui disponíveis.

O anúncio pode incluir:

* identificador da réplica;
* identificador do bucket;
* identificador do objeto;
* versão do objeto;
* lista de fragmentos disponíveis;
* hashes ou referências de integridade;
* data de sincronização;
* validade da disponibilidade;
* estado de saúde da réplica;
* capacidade disponível;
* métricas resumidas.

O Origin não deve confiar cegamente no anúncio. A disponibilidade anunciada deve ser usada como informação operacional, sempre subordinada à autorização, política e validação de integridade feita pelo SDK.

### 6. Serviço de fragmentos para SDKs

Replica/Edge pode servir fragmentos para SDKs autorizados.

A réplica só deve servir fragmentos quando o solicitante apresentar autorização válida emitida pelo Origin.

A autorização apresentada pelo SDK deve permitir validar:

* objeto solicitado;
* fragmento solicitado;
* escopo da operação;
* validade temporal;
* fonte autorizada;
* política aplicável;
* identidade ou contexto do solicitante, quando necessário.

Replica/Edge deve negar a solicitação quando:

* não houver autorização;
* a autorização estiver expirada;
* a autorização tiver sido revogada;
* o fragmento não estiver no escopo;
* o objeto estiver revogado;
* a réplica não estiver autorizada para servir aquele conteúdo;
* o pacote de acesso for inválido;
* a assinatura ou token não puder ser validado;
* a política aplicável não permitir serviço pela réplica.

### 7. Recebimento de revogações

Replica/Edge deve receber e aplicar revogações emitidas pelo Origin.

Revogações podem afetar:

* réplica;
* bucket;
* objeto;
* versão;
* fragmento;
* usuário;
* aplicação;
* pacote de acesso;
* política;
* fonte autorizada.

Ao receber uma revogação, a réplica deve:

* interromper novas sincronizações afetadas;
* deixar de anunciar disponibilidade do conteúdo revogado;
* deixar de servir fragmentos afetados;
* atualizar estado local;
* registrar evento de auditoria;
* reportar aplicação da revogação ao Origin, quando exigido pela política.

A revogação não precisa prometer apagamento físico imediato em todos os casos, mas deve impedir novos serviços autorizados a partir da réplica.

### 8. Reporte de saúde

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
* estado da fila de sincronização;
* quantidade de objetos ou fragmentos disponíveis;
* revogações pendentes ou aplicadas.

Essas informações ajudam o Origin a decidir se a réplica deve ou não aparecer como fonte elegível para SDKs.

### 9. Reporte de métricas

Replica/Edge deve reportar métricas operacionais ao Origin.

Métricas recomendadas:

* bytes sincronizados a partir do Origin;
* bytes servidos para SDKs;
* fragmentos sincronizados;
* fragmentos servidos;
* objetos sincronizados;
* objetos atendidos;
* falhas de autenticação;
* falhas de autorização;
* falhas de sincronização;
* tentativas de acesso negadas;
* solicitações com autorização expirada;
* solicitações para objetos revogados;
* fragmentos inválidos detectados;
* revogações recebidas;
* revogações aplicadas;
* tempo médio de resposta;
* vazão média;
* taxa de erro.

Essas métricas devem contribuir para avaliar disponibilidade, desempenho e redução de carga no Origin.

## Endpoints conceituais

Os endpoints abaixo são conceituais e podem ser ajustados durante a implementação.

### Autenticar ou registrar réplica

```http
POST /pontemesh/replicas/register
```

Registra ou provisiona uma Replica/Edge no Origin.

Deve exigir autenticação administrativa ou fluxo seguro de provisionamento.

### Consultar plano de sincronização

```http
GET /pontemesh/replicas/{replicaId}/sync-plan
```

Retorna o plano de sincronização autorizado para a réplica.

### Baixar fragmento autorizado do Origin

```http
GET /pontemesh/replicas/{replicaId}/objects/{objectId}/fragments/{fragmentId}
```

Permite que a réplica baixe um fragmento autorizado a partir do Origin.

A operação deve validar identidade da réplica, escopo, política e validade do plano de sincronização.

### Anunciar disponibilidade

```http
POST /pontemesh/replicas/{replicaId}/availability
```

Permite que a réplica informe ao Origin quais objetos ou fragmentos possui localmente.

### Reportar saúde

```http
POST /pontemesh/replicas/{replicaId}/health
```

Permite que a réplica reporte seu estado operacional.

### Reportar métricas

```http
POST /pontemesh/replicas/{replicaId}/metrics
```

Permite que a réplica envie métricas de sincronização, serviço de fragmentos, falhas e uso de recursos.

### Receber revogações ou mudanças de política

```http
GET /pontemesh/replicas/{replicaId}/policy-updates
```

Permite que a réplica consulte revogações e mudanças de política pendentes.

Também pode ser substituído ou complementado por outro mecanismo seguro de notificação, desde que autenticado, autorizado e auditável.

### Servir fragmento para SDK autorizado

```http
GET /replica/fragments/{fragmentId}
```

Endpoint exposto pela Replica/Edge para servir fragmentos a SDKs autorizados.

A solicitação deve apresentar autorização válida emitida pelo Origin.

A réplica deve validar o escopo antes de retornar qualquer dado.

## Regras de segurança

Replica/Edge deve seguir as seguintes regras:

* só deve servir fragmentos quando o solicitante apresentar autorização válida emitida pelo Origin;
* não deve confiar em pacote de acesso expirado;
* não deve aceitar autorização emitida por entidade diferente do Origin;
* não deve emitir autorização própria;
* não deve aceitar upload arbitrário de clientes;
* não deve anunciar fragmentos que não foram sincronizados ou autorizados;
* não deve continuar servindo objeto revogado;
* não deve servir fragmentos fora do escopo autorizado;
* deve validar integridade do que sincroniza;
* deve registrar falhas de autenticação;
* deve registrar falhas de autorização;
* deve registrar tentativas de uso de autorização expirada;
* deve registrar tentativas de acesso fora de escopo;
* deve aplicar revogações recebidas do Origin;
* deve usar bibliotecas consolidadas para autenticação, assinatura, tokens, mTLS, criptografia e comparação segura.

## Validação de integridade

Replica/Edge deve validar a integridade dos dados sincronizados a partir do Origin.

A validação deve considerar:

* hash esperado no manifesto;
* tamanho esperado;
* intervalo de bytes;
* versão do objeto;
* identificação do fragmento;
* política aplicável.

Fragmentos inválidos devem ser descartados.

Fragmentos sincronizados corretamente ainda deverão ser validados pelo SDK quando forem consumidos, pois a validação pelo SDK é parte da segurança do plano de dados.

## Modelo de confiança

Replica/Edge é mais estável que peers comuns, mas não deve ser tratada como totalmente confiável.

O modelo correto é:

* Origin é autoridade;
* Replica/Edge é fonte auxiliar autorizada;
* SDK valida fragmentos recebidos;
* Client consome conteúdo por meio do SDK ou API compatível;
* peers e réplicas não são autoridades sobre integridade, autorização ou disponibilidade final.

Mesmo que uma réplica seja comprometida, ela não deve conseguir comprometer a integridade do objeto final, pois o SDK deve validar os fragmentos conforme manifesto autorizado pelo Origin.

## Regras de autorização para servir fragmentos

Antes de servir um fragmento, Replica/Edge deve verificar:

* se a autorização foi emitida pelo Origin;
* se a autorização ainda está vigente;
* se a autorização permite aquele objeto;
* se a autorização permite aquele fragmento;
* se a réplica está entre as fontes autorizadas;
* se o objeto não foi revogado;
* se o pacote de acesso não foi revogado;
* se a política permite serviço por Replica/Edge;
* se o solicitante está dentro do escopo esperado.

Caso qualquer validação falhe, a réplica deve negar a resposta.

## Auditoria

Replica/Edge deve registrar eventos relevantes para rastreabilidade.

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

Os logs não devem expor segredos, tokens completos, chaves privadas, tickets sensíveis ou URLs temporárias completas.

## Relação com o Origin

Toda operação relevante da Replica/Edge deve estar subordinada ao Origin.

O Origin deve decidir:

* quais réplicas são válidas;
* quais réplicas estão revogadas;
* quais objetos podem ser sincronizados;
* quais fragmentos podem ser servidos;
* quais políticas estão vigentes;
* quais réplicas podem aparecer como fontes elegíveis para o SDK;
* quando uma réplica deve parar de servir determinado conteúdo.

Replica/Edge não deve substituir o Origin. Ela apenas reforça o plano de dados.

## Relação com o SDK

O SDK pode obter fragmentos da Replica/Edge quando o pacote de acesso emitido pelo Origin permitir.

O SDK deve:

* apresentar autorização válida à Replica/Edge;
* validar fragmentos recebidos;
* reportar falhas quando aplicável;
* acionar fallback quando a réplica falhar, negar acesso ou apresentar desempenho inadequado.

Replica/Edge deve negar solicitações que não estejam de acordo com o pacote de acesso e a política vigente.

## Comportamento em falhas

A Replica/Edge pode falhar, ficar indisponível ou ser removida das fontes elegíveis.

Nesses casos:

* o SDK deve tentar outra fonte autorizada;
* o SDK pode recorrer ao Origin;
* o Origin deve registrar indisponibilidade ou falha da réplica;
* a réplica pode ser colocada em observação ou removida temporariamente;
* falhas repetidas devem afetar sua elegibilidade como fonte;
* revogação deve impedir uso futuro até nova autorização.

A falha de Replica/Edge não deve impedir o funcionamento do sistema, pois o Origin continua sendo fonte final de garantia.

## Síntese

Replica/Edge é uma fonte auxiliar autorizada para reforçar a distribuição de fragmentos.

Ela deve autenticar-se com o Origin, sincronizar apenas conteúdo autorizado, validar integridade, anunciar disponibilidade, servir fragmentos somente para SDKs autorizados, aplicar revogações e reportar métricas.

Replica/Edge não emite autorização própria, não substitui o Origin e não deve ser tratada como autoridade de segurança.

O Origin continua sendo o centro de controle da arquitetura.
