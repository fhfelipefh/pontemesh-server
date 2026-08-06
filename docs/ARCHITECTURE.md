# Arquitetura

Este documento consolida a arquitetura proposta para o repositório `pontemesh-server`, contemplando os papéis **Origin** e **Replica/Edge**, além dos contratos necessários para integração com SDKs e aplicações cliente.

## Visão geral

O **Ponte Mesh** é um framework para distribuição híbrida de conteúdo digital. A arquitetura preserva o **Origin** como autoridade central do sistema, responsável pelo plano de controle, enquanto o plano de dados pode combinar diferentes fontes de entrega conforme autorização, disponibilidade, segurança e desempenho.

O plano de dados pode utilizar:

* entrega direta pelo Origin;
* entrega por nós Replica/Edge;
* entrega por peers autorizados via SDK;
* fallback automático para o Origin quando fontes auxiliares não forem aplicáveis.

O P2P é mecanismo de aceleração e redução de carga, subordinado à autorização, ao manifesto, às políticas e à revogação emitidas pelo Origin. A entrega direta pelo Origin é o comportamento padrão quando não há fontes auxiliares elegíveis.

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

Quando não há peers ou réplicas disponíveis, a entrega ocorre diretamente pelo Origin.

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

O Origin é o núcleo da arquitetura e permanece capaz de entregar conteúdo diretamente.

### Replica/Edge

O **Replica/Edge** é um nó servidor auxiliar, mais estável que peers comuns, utilizado para aumentar a disponibilidade do plano de dados e reduzir a dependência exclusiva do Origin em cenários de alta demanda.

Ele replica conteúdos a partir do Origin conforme políticas definidas, considerando fatores como demanda, recorrência de acesso, validade temporal, tamanho do objeto, custo estimado de redistribuição e relevância operacional.

Toda comunicação entre Origin e Replica/Edge deve ser autenticada, autorizada, auditável e revogável. Replica/Edge opera dentro das regras emitidas pelo Origin.

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
* operar diretamente com o Origin quando necessário;
* revalidar expiração e revogação em transferências prolongadas.

O SDK trata a distribuição híbrida como otimização controlada.

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

O pacote de acesso pode indicar peers, réplicas ou obtenção direta pelo Origin.

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

O SDK obtém fragmentos diretamente do Origin quando essa for a fonte elegível.

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

A seleção de fontes preserva o Origin como fonte elegível de garantia.

## Replicação

Replica/Edge utiliza replicação seletiva conforme política.

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

## Relação com a API S3-like

A API S3-like deve cobrir as operações fundamentais de buckets e objetos, como envio, leitura, listagem, consulta de metadados, recuperação parcial e remoção lógica.

Essas operações são atendidas diretamente pelo Origin.

As funcionalidades específicas da arquitetura híbrida, como políticas de fragmentação, priorização, fallback, Replica/Edge, métricas, auditoria e fontes autorizadas, devem ser expostas por APIs próprias do Ponte Mesh quando não se encaixarem naturalmente no modelo S3.

## Limites assumidos

A arquitetura assume os seguintes limites:

* o Origin precisa estar acessível no início da operação;
* P2P depende de NAT, firewall, churn, densidade de peers e restrições do ambiente;
* Replica/Edge aumenta disponibilidade sob controle central;
* revogação impede novas autorizações, mas não garante apagamento físico imediato de cópias transitórias já distribuídas;
* peers exigem validação de fragmentos;
* fragmentos devem ser validados antes de serem aceitos;
* o objetivo é reduzir a carga do Origin quando possível;
* a distribuição híbrida deve ser usada apenas quando for autorizada, segura e tecnicamente vantajosa;

## Síntese

A arquitetura do Ponte Mesh separa controle e transferência.

O **Origin** centraliza o controle, a autorização, o catálogo, a disponibilidade e a revogação.

O **plano de dados** pode usar Origin, Replica/Edge e peers autorizados para distribuir fragmentos.

O **SDK** esconde a complexidade da obtenção híbrida e garante validação, seleção de fontes e fallback.

O **P2P** é um mecanismo de aceleração subordinado ao controle central.

O **Replica/Edge** é um reforço de disponibilidade autorizado pelo Origin.

O **Origin** atende diretamente quando a distribuição híbrida não for aplicável.
