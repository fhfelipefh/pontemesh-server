# Arquitetura

Este documento consolida a arquitetura proposta para o repositório `pontemesh-server`, contemplando os papéis **Origin** e **Replica/Edge**, além dos contratos necessários para integração com SDKs e aplicações cliente.

## Visão geral

O **Ponte Mesh** é um framework para distribuição híbrida de conteúdo digital. A arquitetura preserva o **Origin** como autoridade central do sistema, responsável pelo plano de controle, enquanto o plano de dados pode combinar diferentes fontes de entrega conforme autorização, disponibilidade, segurança e desempenho.

O plano de dados pode utilizar:

* entrega direta pelo Origin;
* entrega por nós Replica/Edge;
* entrega por peers autorizados via SDK;
* fallback automático para o Origin quando a distribuição por fontes auxiliares falhar, não estiver disponível ou não for tecnicamente vantajosa.

A proposta não substitui o controle central por P2P. O P2P deve ser tratado como mecanismo de aceleração, redução de carga e aproveitamento de fontes auxiliares, sempre subordinado à autorização, ao manifesto, às políticas e à revogação emitidas pelo Origin.

A ausência de peers não representa falha do servidor nem exige um modo especial de execução. Nesse cenário, o Origin continua cumprindo sua função principal, servindo os objetos diretamente e mantendo o controle sobre autenticação, autorização, disponibilidade, manifestos, métricas e revogação.

## Planos da arquitetura

### Plano de controle

O plano de controle é responsabilidade do **Origin**.

Suas principais responsabilidades são:

* ingestão e armazenamento primário;
* catálogo de buckets, objetos, versões e metadados;
* autenticação de usuários, aplicações, SDKs e réplicas;
* autorização prévia de acesso;
* emissão de pacote de acesso com manifesto, credenciais temporárias, políticas e fontes autorizadas;
* revogação, expiração e deleção lógica;
* seleção, registro ou anúncio de fontes elegíveis;
* métricas e auditoria;
* integração administrativa via painel, API e MCP.

O Origin deve participar do início de toda obtenção de conteúdo. Sem consulta autorizada ao Origin, SDKs, peers e réplicas não devem obter nem servir conteúdo.

Essa decisão preserva controle centralizado sobre publicação, autenticação, autorização, disponibilidade e revogação, mesmo quando a transferência dos dados ocorrer por fontes auxiliares.

### Plano de dados

O plano de dados é responsável pela transferência efetiva dos fragmentos.

Ele pode operar de forma híbrida, combinando:

* Origin como fonte direta e fonte final de garantia;
* Replica/Edge como fonte auxiliar de fragmentos autorizados e sincronizados;
* peers autorizados como fontes temporárias de fragmentos, quando a política permitir;
* fallback para o Origin quando fontes distribuídas falharem, expirarem ou apresentarem desempenho inadequado.

O SDK deve validar cada fragmento por hash antes de aceitá-lo. Sempre que possível, o fallback deve ocorrer no nível do fragmento, preservando o progresso já validado e evitando reiniciar a obtenção completa do objeto.

Quando não houver peers ou réplicas disponíveis, o plano de dados continua funcional por meio do Origin. Nesse caso, a entrega ocorre diretamente pelo servidor de origem, sem comprometer o modelo arquitetural.

## Componentes

### Origin

O **Origin** é a autoridade central do sistema.

Ele deve implementar o subconjunto **S3-like**, os endpoints próprios de controle do framework, os contratos para Replica/Edge, a emissão de manifestos, os pacotes de acesso e os mecanismos de segurança.

Responsabilidades principais:

* receber objetos;
* armazenar conteúdo primário;
* manter catálogo e metadados;
* autenticar e autorizar acessos;
* gerar manifestos;
* emitir pacotes de acesso;
* controlar disponibilidade;
* aplicar expiração e revogação;
* fornecer objetos e fragmentos diretamente quando necessário;
* atuar como fallback para obtenções híbridas;
* registrar métricas e auditoria;
* expor APIs administrativas e operacionais.

O Origin não é apenas uma alternativa de fallback. Ele é o núcleo da arquitetura e deve permanecer capaz de entregar conteúdo mesmo quando não houver peers, réplicas ou fontes auxiliares disponíveis.

### Replica/Edge

O **Replica/Edge** é um nó servidor auxiliar, mais estável que peers comuns, utilizado para aumentar a disponibilidade do plano de dados e reduzir a dependência exclusiva do Origin em cenários de alta demanda.

Ele replica conteúdos a partir do Origin conforme políticas definidas, considerando fatores como demanda, recorrência de acesso, validade temporal, tamanho do objeto, custo estimado de redistribuição e relevância operacional.

A réplica não deve ser tratada como confiável apenas por estar em infraestrutura própria. Toda comunicação entre Origin e Replica/Edge deve ser autenticada, autorizada, auditável e revogável.

O Replica/Edge não atua como autoridade independente. Ele deve operar dentro das regras emitidas pelo Origin.

### SDK

O **SDK** é um componente externo a este repositório, mas o servidor deve fornecer contratos estáveis para sua implementação em diferentes plataformas.

O SDK consome os contratos do Origin, interpreta manifestos, mantém o mapa local de fragmentos, seleciona fontes, valida integridade, controla progresso e aciona fallback quando necessário.

Responsabilidades esperadas do SDK:

* consultar obrigatoriamente o Origin antes da obtenção;
* receber e interpretar o pacote de acesso;
* processar manifestos;
* selecionar fragmentos;
* selecionar fontes autorizadas;
* validar fragmentos por hash;
* preservar fragmentos já validados;
* trocar de fonte em caso de falha;
* acionar fallback para o Origin quando necessário;
* operar diretamente com o Origin quando não houver peers ou réplicas disponíveis;
* revalidar expiração e revogação em transferências prolongadas.

O SDK não deve assumir que sempre existirá uma malha distribuída disponível. A distribuição híbrida é uma otimização controlada, não uma dependência obrigatória para o funcionamento do servidor.

### Client

O **Client** é a aplicação consumidora dos objetos digitais.

Ele deve interagir com uma interface de alto nível, preferencialmente compatível com o modelo **S3-like** nas operações fundamentais de objeto, sem lidar diretamente com descoberta de peers, validação de fragmentos, seleção de fontes ou fallback.

Quando permitido pelas políticas do Origin, o Client pode colaborar temporariamente com fragmentos já obtidos, por meio do SDK.

## Artefatos lógicos

A arquitetura utiliza alguns artefatos lógicos principais.

### Manifesto

Estrutura que descreve o objeto fragmentado.

Deve conter, no mínimo:

* identificação do objeto;
* versão;
* lista de fragmentos;
* índice de cada fragmento;
* tamanho esperado;
* intervalo de bytes;
* hash de integridade;
* metadados necessários para reconstrução;
* política aplicável;
* informações de disponibilidade.

### Pacote de acesso

Autorização temporária emitida pelo Origin.

Pode conter:

* manifesto autorizado;
* credencial ou ticket temporário;
* prazo de expiração;
* fontes autorizadas;
* política de seleção de fragmentos;
* política de seleção de fontes;
* endpoints de fallback;
* restrições aplicáveis ao acesso.

O pacote de acesso pode indicar peers e réplicas quando existirem fontes auxiliares elegíveis. Caso não existam, o pacote ainda pode autorizar a obtenção direta pelo Origin.

### Estado de disponibilidade

Representa a condição atual do objeto dentro do sistema.

Estados conceituais possíveis:

* disponível;
* expirado;
* revogado;
* indisponível;
* bloqueado.

### Fonte autorizada

Representa uma fonte habilitada pelo Origin para participar da obtenção.

Tipos possíveis:

* Origin;
* Replica/Edge;
* peer autorizado.

Cada fonte deve possuir escopo, validade e condições de uso.

O Origin sempre deve ser considerado a fonte de referência e garantia. Replica/Edge e peers são fontes auxiliares condicionadas à autorização, disponibilidade e política aplicável.

### Mapa de progresso

Estrutura mantida pelo SDK para controlar o estado local dos fragmentos.

Estados conceituais possíveis:

* `PENDING`;
* `DOWNLOADING`;
* `VALIDATED`;
* `FAILED`;
* `INVALID`;
* `FALLBACK`.

Fragmentos marcados como `VALIDATED` não devem ser baixados novamente.

## Fluxo de obtenção

1. O Client solicita um objeto ao SDK.
2. O SDK consulta obrigatoriamente o Origin.
3. O Origin autentica a requisição, autoriza o acesso e avalia as políticas aplicáveis.
4. O Origin retorna um pacote de acesso contendo manifesto, fontes autorizadas, política de obtenção e fallback.
5. O SDK interpreta o manifesto e classifica os fragmentos.
6. O SDK seleciona fontes elegíveis para cada fragmento.
7. O SDK baixa fragmentos de peers autorizados, Replica/Edge ou Origin.
8. Cada fragmento recebido é validado por hash.
9. Fragmentos válidos são persistidos localmente e marcados como `VALIDATED`.
10. Fragmentos inválidos, incompletos ou expirados são descartados ou recolocados na fila.
11. Falhas acionam troca de fonte ou fallback por fragmento.
12. Transferências prolongadas podem revalidar expiração, revogação e disponibilidade.
13. O SDK remonta logicamente o objeto apenas com fragmentos validados.
14. O objeto final é entregue ao Client.

Se não houver peers ou Replica/Edge disponíveis, o fluxo permanece válido. O SDK obtém os fragmentos diretamente do Origin, mantendo as mesmas regras de autorização, manifesto e validação.

## Seleção de fontes

A ordem conceitual padrão de preferência é:

1. `PEER`, quando autorizado, estável e com o fragmento necessário;
2. `REPLICA_EDGE`, quando autorizado, autenticado e com o fragmento necessário;
3. `ORIGIN`, como fonte direta e fonte final de garantia.

O SDK deve considerar, no mínimo:

* disponibilidade do fragmento;
* autorização da fonte;
* expiração da autorização;
* vazão estimada;
* latência média;
* taxa de sucesso;
* falhas recentes;
* estado de circuito da fonte;
* política retornada pelo Origin.

O objetivo da seleção de fontes é aproveitar P2P e Replica/Edge quando houver benefício técnico, mas sem comprometer previsibilidade, segurança ou continuidade da obtenção.

A seleção de fontes não deve tornar peers ou réplicas obrigatórios. O servidor deve continuar funcional mesmo quando a única fonte disponível for o Origin.

## Replicação

Replica/Edge deve utilizar replicação seletiva, não cópia indiscriminada de todos os objetos.

Critérios possíveis:

* maior demanda;
* recorrência de acesso;
* relevância operacional;
* validade temporal;
* tamanho do objeto;
* custo estimado de redistribuição;
* política de bucket;
* política de objeto;
* disponibilidade desejada;
* comportamento histórico de acesso.

Toda replicação deve ser autenticada, autorizada, auditável e revogável.

O Origin deve continuar sendo a autoridade sobre quais objetos podem ser replicados, por quanto tempo, sob quais condições e para quais réplicas.

A inexistência de réplicas não impede o funcionamento do servidor. Ela apenas reduz as possibilidades de entrega auxiliar no plano de dados.

## Relação com a API S3-like

A API S3-like deve cobrir as operações fundamentais de buckets e objetos, como envio, leitura, listagem, consulta de metadados, recuperação parcial e remoção lógica.

Essas operações devem funcionar mesmo sem peers ou Replica/Edge, pois o Origin é capaz de atender diretamente as requisições base.

As funcionalidades específicas da arquitetura híbrida, como políticas de fragmentação, priorização, fallback, Replica/Edge, métricas, auditoria e fontes autorizadas, devem ser expostas por APIs próprias do Ponte Mesh quando não se encaixarem naturalmente no modelo S3.

## Limites assumidos

A arquitetura assume os seguintes limites:

* o Origin precisa estar acessível no início da operação;
* P2P pode não estar disponível por NAT, firewall, churn, baixa densidade de peers ou restrições do ambiente;
* Replica/Edge aumenta disponibilidade, mas não elimina a necessidade de controle central;
* revogação impede novas autorizações, mas não garante apagamento físico imediato de cópias transitórias já distribuídas;
* peers não devem ser tratados como fontes confiáveis sem validação;
* fragmentos devem ser validados antes de serem aceitos;
* o objetivo é reduzir a carga do Origin quando possível, não eliminar o Origin;
* a distribuição híbrida deve ser usada apenas quando for autorizada, segura e tecnicamente vantajosa;
* a ausência de peers ou réplicas não caracteriza falha, apenas faz com que a entrega ocorra diretamente pelo Origin.

## Síntese

A arquitetura do Ponte Mesh separa controle e transferência.

O **Origin** centraliza o controle, a autorização, o catálogo, a disponibilidade e a revogação.

O **plano de dados** pode usar Origin, Replica/Edge e peers autorizados para distribuir fragmentos.

O **SDK** esconde a complexidade da obtenção híbrida e garante validação, seleção de fontes e fallback.

O **P2P** é um mecanismo de aceleração, não uma substituição do controle central.

O **Replica/Edge** é um reforço de disponibilidade, não uma autoridade independente.

O **Origin** continua plenamente funcional mesmo sem peers ou réplicas, pois a distribuição híbrida é uma otimização controlada, não uma condição obrigatória para o servidor cumprir sua finalidade.
