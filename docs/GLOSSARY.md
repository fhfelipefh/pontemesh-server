# Glossário

Este documento reúne os principais termos utilizados na arquitetura do Ponte Mesh.

O objetivo do glossário é padronizar a linguagem do projeto, evitando ambiguidades durante a implementação, documentação, revisão arquitetural e geração de código com apoio de IA.

## Origin

Servidor central e autoridade principal do sistema.

O Origin é responsável pelo plano de controle, incluindo ingestão, armazenamento primário, catálogo, autenticação, autorização, emissão de pacotes de acesso, geração de manifestos, controle de disponibilidade, revogação, expiração, métricas, auditoria e fallback.

Quando não há fontes auxiliares elegíveis, o Origin entrega objetos diretamente.

## Replica/Edge

Nó servidor auxiliar, mais estável que peers comuns, utilizado para reforçar a disponibilidade do plano de dados.

O Replica/Edge pode replicar objetos ou fragmentos autorizados a partir do Origin e auxiliar na entrega de conteúdo para clientes.

Toda operação de Replica/Edge é autenticada, autorizada, auditável e revogável pelo Origin.

## SDK

Biblioteca externa consumida por aplicações cliente.

O SDK abstrai a complexidade da obtenção híbrida, sendo responsável por consultar o Origin, obter pacote de acesso, interpretar manifestos, selecionar fragmentos, escolher fontes autorizadas, validar integridade, controlar progresso e acionar fallback quando necessário.

O SDK obtém conteúdo com autorização prévia do Origin.

## Client

Aplicação consumidora do conteúdo.

O Client utiliza o SDK ou a API compatível para obter objetos digitais. Descoberta de peers, validação de fragmentos, seleção de fontes, fallback e reconstrução ficam no SDK.

Quando permitido pelas políticas do Origin, o Client pode colaborar temporariamente compartilhando fragmentos já obtidos por meio do SDK.

## Peer

Cliente autorizado que pode compartilhar temporariamente fragmentos já obtidos.

Todo fragmento recebido de um peer deve ser validado por integridade antes de ser aceito.

A participação de peers depende de autorização, política aplicável, disponibilidade, conectividade e condições técnicas adequadas.

## Plano de controle

Parte da arquitetura responsável pelas decisões, permissões e políticas do sistema.

Inclui autenticação, autorização, catálogo, manifesto, emissão de pacotes de acesso, políticas de distribuição, seleção ou anúncio de fontes autorizadas, revogação, expiração, métricas, auditoria e coordenação operacional.

No Ponte Mesh, o plano de controle é centralizado no Origin.

## Plano de dados

Parte da arquitetura responsável pela transferência efetiva dos dados.

Pode envolver Origin, Replica/Edge e peers autorizados. O plano de dados opera sobre fragmentos, permitindo que diferentes partes de um objeto sejam obtidas de fontes distintas.

Mesmo quando o plano de dados utiliza fontes auxiliares, ele continua subordinado às regras emitidas pelo Origin.

## Objeto

Unidade lógica de conteúdo armazenada e distribuída pelo sistema.

Um objeto pode representar um arquivo, vídeo, imagem, áudio, pacote, documento ou qualquer conteúdo digital identificado por bucket e chave, seguindo inspiração no modelo S3-like.

Internamente, um objeto pode ser dividido em fragmentos para permitir distribuição híbrida, validação parcial e fallback por partes.

## Bucket

Contêiner lógico utilizado para organizar objetos.

O conceito é inspirado no modelo S3. Políticas de acesso, distribuição, retenção, expiração, replicação e auditoria podem ser aplicadas em nível de bucket.

## Chave do objeto

Identificador lógico de um objeto dentro de um bucket.

A chave permite localizar, recuperar, listar, versionar ou remover logicamente um objeto.

## API S3-like

Interface inspirada no modelo S3 para operações fundamentais de buckets e objetos.

Deve cobrir operações como criar e listar buckets, enviar objeto, listar objetos, consultar metadados, recuperar objeto, recuperar intervalos de bytes, remover logicamente objeto e gerar URL temporária ou mecanismo equivalente.

A API S3-like cobre operações base de objeto. Funcionalidades específicas da arquitetura ficam na API Ponte Mesh.

## API Ponte Mesh

Conjunto de APIs próprias para recursos que não se encaixam naturalmente no modelo S3.

Pode incluir contratos para manifestos, pacotes de acesso, políticas de fragmentação, seleção de fontes, fallback, Replica/Edge, métricas, auditoria, revogação, disponibilidade, SDKs e dashboard administrativo.

## Manifesto

Documento lógico que descreve a estrutura de um objeto fragmentado.

Deve conter informações como identificação do objeto, versão, lista de fragmentos, índices, intervalos de bytes, tamanhos esperados, hashes de integridade, metadados de reconstrução, política aplicável e informações de disponibilidade.

O manifesto orienta o SDK sobre como obter, validar e remontar o objeto.

## Pacote de acesso

Autorização temporária emitida pelo Origin para uma operação de obtenção.

Pode conter manifesto autorizado, credencial ou ticket temporário, prazo de expiração, fontes autorizadas, políticas de seleção, endpoints de fallback e restrições aplicáveis ao acesso.

O pacote de acesso define o que o SDK pode fazer durante uma obtenção específica.

## Fragmento

Parte lógica de um objeto.

Cada fragmento deve possuir identificação, posição no objeto, tamanho esperado, intervalo de bytes e hash de integridade.

Fragmentos permitem obtenção paralela, distribuição por múltiplas fontes, validação parcial, recuperação por intervalo e fallback sem reiniciar o download completo.

## Hash de integridade

Valor calculado sobre um fragmento ou objeto para verificar se os dados recebidos correspondem ao conteúdo esperado.

O SDK deve validar fragmentos por hash antes de aceitá-los. Dados inválidos, incompletos ou adulterados devem ser descartados.

## Fonte autorizada

Fonte incluída pelo Origin no pacote de acesso e permitida para participar da obtenção.

Tipos possíveis:

* Origin;
* Replica/Edge;
* peer autorizado.

Uma fonte autorizada deve possuir escopo, validade, permissões e condições de uso.

## Fallback

Troca automática para uma fonte mais confiável quando a fonte atual falha, expira, apresenta baixa qualidade ou não possui o fragmento necessário.

Normalmente, o Origin é a fonte final de garantia.

Sempre que possível, o fallback deve ocorrer por fragmento, preservando fragmentos já validados e evitando reiniciar a obtenção completa do objeto.

## Fallback por fragmento

Estratégia em que apenas o fragmento problemático é redirecionado para outra fonte, em vez de reiniciar todo o objeto.

Essa abordagem reduz desperdício de banda, preserva progresso validado e melhora a tolerância a falhas em ambientes distribuídos.

## Revogação

Ação que bloqueia novas autorizações de acesso.

A revogação pode afetar objetos, usuários, aplicações, pacotes de acesso, réplicas, peers ou políticas específicas.

Em transferências prolongadas, o SDK pode revalidar o estado do acesso e interromper a obtenção caso a autorização tenha sido revogada.

## Expiração

Fim da validade de uma autorização, pacote de acesso, URL temporária, política ou credencial.

Após a expiração, a fonte ou cliente não deve continuar operando com a autorização antiga.

## Deleção lógica

Marcação de um objeto como removido ou indisponível sem pressupor apagamento físico imediato de todas as cópias transitórias já distribuídas.

A deleção lógica deve impedir novas autorizações e novas obtenções controladas pelo Origin.

## Catálogo

Registro mantido pelo Origin com informações sobre buckets, objetos, versões, metadados, estados de disponibilidade, políticas, localização lógica, replicação, revogação e auditoria.

O catálogo é parte essencial do plano de controle.

## Metadados

Informações descritivas associadas a buckets, objetos, fragmentos, versões ou transferências.

Podem incluir tamanho, tipo de conteúdo, data de criação, data de modificação, versão, política aplicável, estado de disponibilidade, hashes, origem e atributos administrativos.

## Estado de disponibilidade

Condição atual de um objeto ou fragmento dentro do sistema.

Estados conceituais possíveis:

* disponível;
* expirado;
* revogado;
* indisponível;
* bloqueado.

Esse estado orienta o Origin, o SDK e as fontes auxiliares sobre a possibilidade de obtenção ou compartilhamento.

## Mapa de progresso

Estrutura local mantida pelo SDK para acompanhar o estado de cada fragmento durante a obtenção.

Estados conceituais possíveis:

* `PENDING`;
* `DOWNLOADING`;
* `VALIDATED`;
* `FAILED`;
* `INVALID`;
* `FALLBACK`.

Fragmentos marcados como `VALIDATED` não devem ser baixados novamente.

## `PENDING`

Estado de um fragmento que ainda não foi solicitado ou que voltou para a fila após uma falha recuperável.

## `DOWNLOADING`

Estado de um fragmento que está em processo de transferência.

## `VALIDATED`

Estado de um fragmento recebido com sucesso e validado por hash.

Somente fragmentos validados podem ser usados na remontagem lógica do objeto.

## `FAILED`

Estado de um fragmento cuja obtenção falhou por erro de rede, timeout, indisponibilidade da fonte ou outro problema operacional.

## `INVALID`

Estado de um fragmento recebido, mas rejeitado por falha de integridade, tamanho incorreto, conteúdo incompleto ou divergência em relação ao manifesto.

## `FALLBACK`

Estado de um fragmento encaminhado para obtenção por uma fonte mais confiável, normalmente o Origin.

## Circuit breaker

Mecanismo local usado pelo SDK para evitar insistência em fontes instáveis ou com falhas repetidas.

Quando uma fonte apresenta muitas falhas, ela pode ser temporariamente ignorada. Após um intervalo ou mudança de estado, pode voltar a ser testada.

## Seleção de fontes

Processo pelo qual o SDK escolhe de onde baixar cada fragmento.

A seleção pode considerar tipo da fonte, disponibilidade do fragmento, autorização, expiração, vazão estimada, latência média, taxa de sucesso, falhas recentes e estado de circuito.

A ordem conceitual padrão é:

1. peer autorizado;
2. Replica/Edge autorizado;
3. Origin.

O Origin permanece como fonte final de garantia.

## Seleção de fragmentos

Processo pelo qual o SDK decide qual fragmento baixar primeiro.

A política pode priorizar fragmentos iniciais, fragmentos próximos ao ponto de consumo, fragmentos raros, fragmentos restantes ou fragmentos problemáticos.

## `headers-first`

Estratégia conceitual em que metadados, cabeçalhos ou partes iniciais necessárias ao reconhecimento e início do uso do conteúdo recebem prioridade.

Pode ser útil para cenários em que a aplicação precisa iniciar leitura, validação ou preparação antes de obter o objeto completo.

## `priority-first`

Estratégia conceitual em que a ordem dos fragmentos é definida por prioridade operacional.

Pode considerar consumo progressivo, criticidade, política do Origin, tipo de conteúdo ou necessidade da aplicação.

## `rarest-first`

Estratégia inspirada em sistemas P2P na qual fragmentos menos disponíveis entre as fontes autorizadas recebem prioridade.

O objetivo é aumentar a redundância de fragmentos raros e reduzir risco de indisponibilidade futura.

## Endgame

Estratégia usada quando restam poucos fragmentos ou quando há fragmentos problemáticos.

O SDK pode solicitar o mesmo fragmento a mais de uma fonte e aceitar a primeira resposta válida, reduzindo o tempo de conclusão da transferência.

## Replica seletiva

Estratégia em que o Replica/Edge não copia todos os objetos indiscriminadamente.

A replicação pode considerar demanda, recorrência de acesso, relevância operacional, validade temporal, tamanho do objeto, custo de redistribuição e política de bucket ou objeto.

## Política

Conjunto de regras emitidas ou aplicadas pelo Origin.

Pode definir autorização, expiração, revogação, fragmentação, seleção de fontes, fallback, replicação, compartilhamento por peers, métricas, auditoria e comportamento do SDK.

## Política de bucket

Política aplicada a todos ou parte dos objetos de um bucket.

Pode controlar permissões, distribuição híbrida, Replica/Edge, expiração, retenção, revogação, métricas e auditoria.

## Política de objeto

Política aplicada a um objeto específico.

Pode sobrescrever ou complementar regras de bucket, definindo comportamento próprio de autorização, fragmentação, obtenção, replicação, fallback e expiração.

## Autorização temporária

Permissão com prazo de validade emitida pelo Origin.

Pode ser representada por ticket, token, credencial temporária, URL assinada ou mecanismo equivalente.

## URL temporária

URL com prazo de validade usada para permitir acesso controlado a determinado objeto ou recurso sem expor credenciais permanentes.

No Ponte Mesh, pode ser usada como mecanismo compatível ou equivalente ao modelo de URLs pré-assinadas.

## Auditoria

Registro de eventos relevantes para segurança, rastreabilidade e operação.

Pode incluir emissões de pacotes de acesso, revogações, autenticações, falhas, transferências, uso de réplicas, uso de peers, alterações de política e ações administrativas.

## Métricas

Dados coletados para observabilidade e análise operacional.

Podem incluir bytes servidos pelo Origin, bytes servidos por Replica/Edge, bytes servidos por peers, taxa de fallback, latência, vazão, falhas, fragmentos inválidos, disponibilidade, uso por bucket e comportamento de fontes.

## MCP

Interface administrativa e de automação sobre o plano de controle.

No Ponte Mesh, o MCP pode ser usado para consultar catálogo, objetos, métricas, estados de disponibilidade, revogações, estatísticas, políticas e operações administrativas do Origin.

O MCP não faz parte do plano de dados e não deve participar diretamente da transferência de fragmentos.

## Dashboard administrativo

Interface futura para operação e configuração do Ponte Mesh.

Deve consumir APIs próprias do Ponte Mesh para gerenciar políticas, buckets, objetos, Replica/Edge, métricas, auditoria, revogações, estratégias de fallback e configurações avançadas que não pertencem naturalmente ao modelo S3-like.

## Segurança fail-closed

Princípio segundo o qual falhas, ambiguidades ou ausência de configuração segura devem resultar em negação de acesso, e não em permissão implícita.

No Ponte Mesh, configurações inseguras, credenciais inválidas, políticas ausentes ou autorizações expiradas devem bloquear a operação.

## Churn

Entrada e saída frequente de participantes em uma rede distribuída.

No contexto do Ponte Mesh, churn afeta principalmente peers comuns, que podem ficar indisponíveis durante uma transferência.

O sistema deve tolerar churn por meio de seleção de fontes, circuit breaker, Replica/Edge e fallback para o Origin.

## NAT

Mecanismo de tradução de endereços de rede que pode dificultar conexões diretas entre peers.

A presença de NAT é uma das razões pelas quais o P2P não pode ser assumido como sempre disponível.

## Firewall

Mecanismo de controle de tráfego de rede que pode bloquear conexões necessárias para comunicação direta entre peers.

Assim como NAT, firewalls podem limitar a viabilidade da distribuição P2P.

## Fonte final de garantia

Fonte que deve ser capaz de atender à obtenção quando fontes auxiliares não estiverem disponíveis, falharem ou não forem autorizadas.

No Ponte Mesh, essa função pertence ao Origin.

## Distribuição híbrida

Modelo de distribuição que combina entrega centralizada pelo Origin com fontes auxiliares, como Replica/Edge e peers autorizados.

A distribuição híbrida busca reduzir carga do Origin quando possível, sem abrir mão de controle, segurança, autorização, revogação e previsibilidade operacional.

## Conteúdo transitório

Cópia temporária de objeto ou fragmento mantida em peers ou fontes auxiliares durante ou após uma transferência.

Conteúdos transitórios não devem ser confundidos com armazenamento primário. O controle sobre novas autorizações permanece no Origin.

## Ingestão

Processo de recebimento e registro de um objeto pelo Origin.

Pode envolver validação inicial, armazenamento, geração de metadados, fragmentação, criação de manifesto e aplicação de políticas.

## Fragmentação

Processo de dividir logicamente um objeto em partes menores.

A fragmentação permite obtenção paralela, validação por partes, compartilhamento por múltiplas fontes e fallback granular.

## Reconstrução do objeto

Processo pelo qual o SDK remonta logicamente o objeto a partir dos fragmentos validados.

A reconstrução só deve usar fragmentos aceitos após validação de integridade.

## Fonte auxiliar

Fonte que pode contribuir para o plano de dados, mas que não substitui a autoridade do Origin.

Replica/Edge e peers autorizados são fontes auxiliares.

## Fonte primária

Fonte que mantém o conteúdo principal e a autoridade sobre sua disponibilidade.

No Ponte Mesh, a fonte primária é o Origin.

## Fonte confiável

Fonte cuja participação foi autorizada pelo Origin e cujos dados ainda precisam ser validados por integridade.

Mesmo fontes autorizadas não dispensam validação de fragmentos.

## Escopo

Limite de permissão concedido a uma entidade, fonte, credencial ou pacote de acesso.

Pode restringir operações por bucket, objeto, fragmento, tempo, ação, aplicação, usuário ou réplica.

## Credencial de réplica

Credencial específica usada por um Replica/Edge para autenticar-se com o Origin.

Não deve ser compartilhada com usuários, aplicações ou outros tipos de entidade.

## Revalidação

Nova consulta ou verificação realizada durante uma transferência para confirmar se autorização, expiração, revogação ou disponibilidade continuam válidas.

Pode ser necessária em transferências longas ou em cenários de mudança de política.

## Operação autorizada

Operação permitida pelo Origin dentro de um escopo e prazo definidos.

Uma operação autorizada não deve ser confundida com permissão permanente.

## Ponte Mesh

Framework proposto para distribuição híbrida de objetos digitais com controle centralizado pelo Origin, entrega por fragmentos, suporte a fontes auxiliares e fallback automático.

O objetivo do Ponte Mesh é reduzir carga do Origin quando houver condições seguras e vantajosas para distribuição híbrida, preservando controle, segurança e previsibilidade operacional.
