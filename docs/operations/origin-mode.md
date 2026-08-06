# Operação em modo Origin

Este documento descreve a operação do `pontemesh-server` quando executado no papel de **Origin**.

O **Origin** é a autoridade central do Ponte Mesh. Ele concentra o plano de controle, mantém o armazenamento primário, emite autorizações, gera ou disponibiliza manifestos, controla políticas, aplica revogações e atua como fonte direta ou fonte final de garantia no plano de dados.

Quando não há fontes auxiliares elegíveis, o Origin serve objetos diretamente, preservando autenticação, autorização, manifesto, integridade, métricas e auditoria.

## Papel do Origin

O Origin é responsável por garantir que toda obtenção de conteúdo ocorra de forma controlada.

Nenhum SDK, Client, peer ou Replica/Edge deve obter ou servir conteúdo sem autorização prévia emitida pelo Origin.

O Origin atende diretamente quando a distribuição híbrida não for aplicável.

## Responsabilidades

O Origin deve ser responsável por:

* receber ingestão de objetos;
* manter o armazenamento primário;
* manter catálogo de buckets, objetos, versões, metadados e estados de disponibilidade;
* expor o subconjunto de API S3-like para operações fundamentais de buckets e objetos;
* expor APIs próprias do Ponte Mesh para manifestos, pacotes de acesso, políticas, Replica/Edge, métricas, auditoria e revogação;
* gerar ou disponibilizar manifestos;
* autenticar usuários, aplicações, SDKs e réplicas;
* autorizar previamente toda obtenção controlada;
* emitir pacotes de acesso;
* emitir URLs temporárias, tickets ou credenciais equivalentes quando aplicável;
* controlar expiração de pacotes de acesso;
* controlar revogação de objetos, usuários, aplicações, pacotes de acesso e réplicas;
* aplicar deleção lógica;
* controlar políticas de bucket e objeto;
* controlar políticas de distribuição híbrida;
* controlar fontes autorizadas;
* coordenar réplicas autorizadas;
* disponibilizar planos de sincronização para Replica/Edge;
* remover réplicas revogadas ou expiradas das fontes elegíveis;
* servir objetos diretamente quando necessário;
* servir fragmentos ou intervalos de bytes para fallback;
* preservar suporte a recuperação parcial por `Range`;
* registrar métricas operacionais;
* registrar eventos de auditoria;
* fornecer contratos estáveis para SDKs.

## Plano de controle

No papel de Origin, o servidor é responsável pelo plano de controle.

Isso inclui:

* autenticação;
* autorização;
* catálogo;
* metadados;
* políticas;
* manifestos;
* pacotes de acesso;
* fontes autorizadas;
* expiração;
* revogação;
* deleção lógica;
* métricas;
* auditoria;
* coordenação de Replica/Edge;
* integração administrativa via API, painel e MCP.

O plano de controle deve permanecer centralizado no Origin.

Replica/Edge e peers participam do plano de dados sob decisões de autorização, revogação, manifesto e política emitidas pelo Origin.

## Plano de dados

Embora o Origin concentre o plano de controle, ele também participa do plano de dados.

No plano de dados, o Origin pode atuar como:

* fonte direta de objetos;
* fonte direta de fragmentos;
* fonte para recuperação por intervalo de bytes;
* fonte de sincronização para Replica/Edge;
* fonte final de garantia em caso de fallback;
* fonte direta quando necessário.

O objetivo da distribuição híbrida é reduzir carga do Origin quando houver fontes auxiliares autorizadas e vantajosas.

## Ingestão de objetos

A ingestão de objetos deve ocorrer pelo Origin.

Durante a ingestão, o Origin deve:

* validar autenticação;
* validar autorização;
* receber o conteúdo;
* armazenar o objeto no armazenamento primário;
* registrar metadados;
* associar o objeto a um bucket;
* registrar estado inicial de disponibilidade;
* aplicar política de bucket ou objeto;
* preparar dados necessários para manifesto;
* registrar métricas e auditoria quando aplicável.

Clientes enviam objetos ao Origin.

## Catálogo e metadados

O Origin deve manter o catálogo oficial do sistema.

O catálogo deve registrar, no mínimo:

* buckets;
* objetos;
* versões, quando aplicável;
* chaves de objeto;
* tamanho;
* tipo de conteúdo;
* datas relevantes;
* estado de disponibilidade;
* políticas aplicáveis;
* informações de manifesto;
* informações de fragmentação;
* estado de revogação;
* estado de deleção lógica;
* relações com Replica/Edge;
* métricas e eventos associados.

O catálogo é parte essencial do plano de controle.

## Manifestos

O Origin deve gerar, disponibilizar, assinar ou validar manifestos.

O manifesto deve descrever a estrutura do objeto fragmentado e conter informações suficientes para que o SDK consiga:

* identificar o objeto;
* conhecer a versão do objeto;
* listar fragmentos;
* identificar intervalos de bytes;
* conhecer tamanhos esperados;
* validar hashes de integridade;
* reconstruir logicamente o objeto;
* aplicar políticas de obtenção;
* acionar fallback quando necessário.

Manifestos não devem ser definidos por peers ou Replica/Edge como fonte de autoridade.

## Pacotes de acesso

O Origin deve emitir pacotes de acesso antes de qualquer obtenção controlada.

Um pacote de acesso pode conter:

* identificação do objeto;
* manifesto autorizado;
* credencial, ticket ou token temporário;
* prazo de expiração;
* escopo da autorização;
* fontes autorizadas;
* política de seleção de fontes;
* política de seleção de fragmentos;
* endpoints de fallback;
* restrições aplicáveis.

O pacote de acesso deve ser temporário, não adivinhável, escopado e revogável.

O Origin deve negar emissão de pacote quando não houver autenticação válida, autorização suficiente ou política aplicável.

## Autenticação e autorização

O Origin deve autenticar e autorizar entidades antes de permitir operações protegidas.

Entidades possíveis:

* usuários;
* aplicações;
* SDKs;
* Replica/Edge;
* operadores administrativos;
* integrações futuras, como dashboard e MCP.

A autenticação deve usar mecanismos e bibliotecas consolidadas. O projeto não deve implementar autenticação, criptografia, assinatura, geração de tokens ou comparação segura de forma caseira.

A autorização deve considerar:

* identidade;
* escopo;
* ação;
* bucket;
* objeto;
* fragmento, quando aplicável;
* validade temporal;
* política de bucket;
* política de objeto;
* estado de disponibilidade;
* revogação;
* expiração.

## Replica/Edge

O Origin deve coordenar Replica/Edge autorizadas.

Responsabilidades do Origin em relação a réplicas:

* registrar identidade de réplica;
* validar credenciais de réplica;
* autenticar comunicação Origin e Replica/Edge;
* autorizar escopos de sincronização;
* emitir plano de sincronização;
* controlar quais objetos ou fragmentos podem ser replicados;
* receber anúncios de disponibilidade;
* receber métricas de saúde;
* receber métricas de transferência;
* aplicar revogações;
* remover réplicas revogadas, expiradas ou inválidas das fontes elegíveis.

Toda réplica deve possuir identidade e credencial próprias.

O Origin deve negar comunicação de réplica desconhecida, expirada, sem escopo, com credencial inválida ou revogada.

Replica/Edge opera com autorização emitida pelo Origin.

## Fallback

O Origin deve atuar como fonte final de garantia.

Quando peers ou Replica/Edge falharem, expirarem, não tiverem o fragmento solicitado ou não forem vantajosos, o SDK deve poder recorrer ao Origin.

O fallback deve ocorrer preferencialmente por fragmento ou intervalo de bytes, preservando fragmentos já validados.

O Origin deve preservar suporte a `Range requests` para permitir:

* retomada parcial;
* fallback granular;
* recuperação de fragmentos específicos;
* redução de desperdício de banda;
* preservação de progresso validado.

O fallback permanece dentro do escopo autorizado.

## Recuperação direta pelo Origin

O Origin deve conseguir atender diretamente operações de leitura de objetos.

Casos comuns:

* a política pode bloquear distribuição por fontes auxiliares;
* a rede pode impedir P2P por NAT ou firewall;
* o SDK pode decidir que fontes auxiliares não são vantajosas;
* o conteúdo pode ser sensível ou restrito;
* o fallback pode exigir recuperação direta.

A entrega direta pelo Origin é comportamento normal da arquitetura.

## Métricas

O Origin deve registrar métricas suficientes para avaliar desempenho, disponibilidade, segurança e redução de carga.

Métricas esperadas:

* bytes servidos diretamente pelo Origin;
* bytes servidos por Replica/Edge;
* bytes servidos por peers, quando reportados pelo SDK;
* quantidade de pacotes de acesso emitidos;
* quantidade de pacotes de acesso negados;
* taxa de fallback;
* fallback por fragmento;
* fallback total de sessão;
* quantidade de objetos obtidos diretamente pelo Origin;
* quantidade de objetos com participação de fontes auxiliares;
* tentativas por fragmento;
* fragmentos invalidados por hash;
* falhas de autenticação;
* falhas de autorização;
* revogações aplicadas;
* réplicas ativas;
* réplicas revogadas;
* disponibilidade de fontes auxiliares.

O Origin deve registrar bytes servidos diretamente para permitir calcular a redução de carga em comparação com o cenário cliente-servidor tradicional.

## Auditoria

O Origin deve auditar operações sensíveis.

Eventos recomendados:

* ingestão de objeto;
* criação de bucket;
* alteração de política;
* emissão de pacote de acesso;
* negação de pacote de acesso;
* expiração de pacote de acesso;
* revogação de objeto;
* revogação de usuário;
* revogação de aplicação;
* revogação de réplica;
* deleção lógica;
* autenticação de Replica/Edge;
* falha de autenticação;
* falha de autorização;
* emissão de plano de sincronização;
* anúncio de disponibilidade de réplica;
* operação administrativa sensível;
* evento MCP.

A auditoria deve registrar quem executou a ação, quando ocorreu, qual recurso foi afetado, qual política foi aplicada e qual foi o resultado.

Logs e auditoria não devem expor segredos, tokens completos, chaves privadas, tickets sensíveis ou URLs temporárias completas.

## Regras operacionais

O Origin deve seguir as seguintes regras:

* toda obtenção controlada deve começar no Origin;
* toda autorização de acesso deve ser emitida pelo Origin;
* todo manifesto deve ser emitido, assinado ou validado pelo Origin;
* todo pacote de acesso deve possuir escopo e expiração;
* toda réplica deve ter identidade e credencial própria;
* réplica desconhecida deve ser negada;
* réplica expirada deve ser negada;
* réplica revogada deve ser negada;
* réplica sem escopo suficiente deve ser negada;
* objeto revogado não deve gerar novo pacote de acesso;
* objeto removido logicamente não deve gerar nova autorização;
* fallback deve preservar fragmentos já validados sempre que possível;
* recuperação por range deve ser preservada;
* operações administrativas devem ser auditadas;
* configurações inseguras devem negar acesso por padrão;

## Segurança

A operação em modo Origin deve seguir os princípios de segurança definidos no projeto.

Regras importantes:

* negar por padrão;
* usar menor privilégio;
* separar credenciais por tipo de entidade;
* usar autorizações temporárias;
* permitir revogação;
* validar integridade por fragmento;
* auditar operações sensíveis;
* validar fragmentos recebidos de peers;
* manter Replica/Edge sob autoridade do Origin;
* usar bibliotecas e frameworks consolidados de segurança;
* evitar mecanismos caseiros de autenticação, assinatura, token, criptografia ou hashing seguro.

## Relação com o SDK

O Origin deve fornecer contratos estáveis para SDKs.

O SDK deve conseguir:

* solicitar pacote de acesso;
* obter manifesto autorizado;
* consultar fontes autorizadas;
* consultar políticas aplicáveis;
* obter endpoints de fallback;
* reportar métricas;
* revalidar autorização em transferências prolongadas;
* operar diretamente com o Origin quando essa for a fonte elegível.

O SDK obtém conteúdo de peers ou Replica/Edge dentro da autorização emitida pelo Origin.

## Relação com o Client

O Client é a aplicação consumidora.

Do ponto de vista do Client, a integração deve ser de alto nível, preferencialmente familiar ao modelo S3-like para operações fundamentais de objeto.

O Client delega ao SDK:

* descoberta de peers;
* seleção de fontes;
* validação de fragmentos;
* fallback;
* circuit breaker;
* reconstrução do objeto;
* revogação operacional;
* coordenação de réplicas.

Essas responsabilidades pertencem ao Origin e ao SDK.

## Síntese

No papel de Origin, o `pontemesh-server` é o centro de controle do Ponte Mesh.

Ele recebe objetos, mantém catálogo, autentica entidades, autoriza acessos, gera manifestos, emite pacotes de acesso, controla revogação, coordena Replica/Edge, serve fallback e registra métricas e auditoria.

O objetivo é permitir distribuição híbrida quando houver condições seguras e vantajosas, preservando o controle centralizado do Origin.
