# Segurança

Segurança é requisito central do Ponte Mesh.

A arquitetura combina controle centralizado com transferência distribuída. Por isso, todo participante fora do Origin deve ser tratado como potencialmente não confiável, incluindo peers, clientes, SDKs, Replica/Edge e qualquer fonte auxiliar do plano de dados.

O Origin deve permanecer como autoridade central sobre autenticação, autorização, disponibilidade, emissão de pacotes de acesso, manifestos, revogação, expiração, auditoria e políticas.

## Diretriz fundamental

O projeto não deve implementar mecanismos próprios de criptografia, assinatura, geração de tokens, hashing seguro, autenticação ou autorização quando existirem bibliotecas, protocolos e frameworks consolidados para isso.

A implementação deve preferir soluções maduras, revisadas e amplamente utilizadas.

Exemplos de tecnologias e bibliotecas que podem ser consideradas conforme a linguagem, plataforma e contexto:

* TLS e mTLS fornecidos por bibliotecas e runtimes consolidados;
* OAuth 2.0 e OpenID Connect por meio de provedores e bibliotecas maduras;
* JWT, JWS e JWK por bibliotecas JOSE consolidadas;
* HMAC e assinaturas assimétricas usando bibliotecas criptográficas confiáveis;
* gerenciamento de senhas com algoritmos apropriados, como Argon2id, bcrypt ou scrypt, por bibliotecas maduras;
* armazenamento e rotação de segredos por ferramentas adequadas, como vaults, secret managers ou mecanismos seguros da infraestrutura;
* frameworks de segurança consolidados da stack escolhida, como Spring Security em aplicações Java;
* bibliotecas criptográficas reconhecidas, como APIs nativas da plataforma, libsodium, Google Tink, Bouncy Castle ou equivalentes confiáveis, quando aplicável.

Não devem ser criados algoritmos próprios para:

* criptografia;
* assinatura;
* geração de tokens;
* geração de números aleatórios seguros;
* armazenamento de senhas;
* validação de JWT;
* comparação segura de assinaturas;
* controle de sessão;
* derivação de chaves;
* proteção contra replay;
* autorização baseada em escopos.

Quando houver dúvida, a decisão mais segura é usar um protocolo ou biblioteca consolidada, documentar a escolha e evitar implementação manual.

## Objetivos de segurança

Os principais objetivos de segurança são:

* impedir acesso a objetos sem autorização prévia do Origin;
* garantir que toda obtenção de conteúdo comece com consulta autorizada ao Origin;
* manter autenticação e autorização compatíveis com revogação e expiração;
* validar integridade de fragmentos independentemente da fonte;
* impedir que Replica/Edge seja usada como bypass do Origin;
* impedir que peers sejam tratados como fontes confiáveis sem validação;
* impedir uso indevido de pacotes de acesso, tickets ou URLs temporárias;
* reduzir risco de replay em requisições sensíveis;
* auditar ações administrativas, replicação, emissão de pacotes de acesso, revogações e falhas;
* limitar atualizações solicitadas pelo painel ao repositório oficial fixo, com artefato selecionado pela plataforma e validado pelo manifesto, tamanho e SHA-256; um comando externo confiável pode substituir o atualizador interno, mas nunca é recebido do navegador;
* tratar webhooks operacionais como saída de rede administrativamente autorizada: aceitar somente HTTP/HTTPS com host, aplicar timeout e limite de redirecionamentos, não enviar segredos e não registrar a URL completa;
* preservar previsibilidade operacional mesmo com peers instáveis, maliciosos ou indisponíveis;
* garantir que falhas de configuração resultem em negação de acesso, não em permissão implícita.

## Princípios

### Negar por padrão

Sem política explícita, autenticação válida e autorização vigente, a operação deve ser negada.

A ausência de configuração segura não deve liberar acesso.

### Menor privilégio

Credenciais, tokens, tickets e pacotes de acesso devem possuir o menor escopo possível.

Permissões devem ser delimitadas por:

* ação;
* bucket;
* objeto;
* fragmento, quando aplicável;
* usuário;
* aplicação;
* réplica;
* prazo de validade;
* política operacional.

### Credenciais separadas

Usuários, aplicações, SDKs, peers e Replica/Edge não devem compartilhar o mesmo segredo.

Credenciais administrativas devem ser separadas de credenciais operacionais.

Credenciais de Replica/Edge devem ser específicas por réplica.

Aplicações públicas, como launchers desktop, não conseguem manter um segredo
reutilizável dentro do binário. Para conteúdo público, elas devem receber em tempo
de execução uma credencial restrita somente a download. Para conteúdo protegido,
devem autenticar o usuário por um provedor consolidado e trocar essa identidade por
uma credencial curta; um identificador público de cliente não é autenticação.

### Autorização temporária

Tickets, URLs temporárias, credenciais transitórias e pacotes de acesso devem expirar.

Autorizações permanentes devem ser evitadas para operações de obtenção e sincronização.

### Integridade por conteúdo

Um fragmento só deve ser aceito após validação de integridade conforme o manifesto autorizado.

A origem do fragmento não elimina a necessidade de validação.

Fragmentos vindos de peers, Replica/Edge ou Origin devem seguir a mesma regra de integridade.

### Revogabilidade

Acesso, réplica, objeto, usuário, aplicação, pacote de acesso e política devem poder ser revogados.

Revogação deve impedir novas autorizações e remover fontes revogadas da lista de fontes elegíveis.

Na implementação inicial, a revogação de objeto marca o item como `REVOKED` no
catálogo. Esse estado impede novas leituras diretas pelo Origin, emissão de
manifesto, emissão de pacote de acesso e inclusão em sync-plan de Replica/Edge.

Réplicas possuem credenciais próprias e revogáveis. Uma réplica revogada não
consegue mais autenticar chamadas operacionais como consulta de sync-plan.

### Separação entre autenticação e autorização

Autenticação prova identidade.

Autorização decide o que a entidade autenticada pode fazer.

Uma Replica/Edge autenticada ainda precisa estar autorizada para sincronizar, anunciar ou servir determinado conteúdo.

### Segurança fail-closed

Falhas, ambiguidades ou estados incompletos devem resultar em bloqueio.

Exemplos:

* política ausente;
* escopo inválido;
* assinatura inválida;
* token expirado;
* réplica revogada;
* manifesto incompatível;
* pacote de acesso vencido;
* fonte não autorizada;
* hash divergente.

Nesses casos, o sistema deve negar a operação.

## Autenticação e autorização

O sistema deve prever autenticação e autorização para diferentes tipos de entidade:

* usuários;
* aplicações;
* SDKs;
* Replica/Edge;
* operadores administrativos;
* integrações futuras, como painel e MCP.

A implementação deve evitar autenticação caseira.

Sempre que possível, devem ser usados padrões e frameworks consolidados, como OAuth 2.0, OpenID Connect, mTLS, JWT assinado por bibliotecas maduras, tokens opacos com introspecção ou mecanismos equivalentes adequados ao contexto.

## Autenticação Origin <-> Replica/Edge

A autenticação entre Origin e Replica/Edge é requisito explícito deste repositório.

Replica/Edge é um servidor auxiliar de distribuição, mas não pode conversar com o Origin sem identidade própria, credencial válida, escopo explícito e auditoria.

O contrato inicial implementa credencial Bearer própria por réplica, armazenada
somente como hash no catálogo. Essa credencial é separada da sessão
administrativa e das credenciais de aplicação/SDK.

A chamada operacional de Replica/Edge também usa assinatura de requisição com
timestamp e nonce. O Origin valida a janela temporal, rejeita nonce repetido e
audita falhas de autenticação.

A comunicação entre Origin e Replica/Edge deve exigir:

* registro de identidade de cada réplica;
* credencial, certificado ou chave específica por réplica;
* autenticação forte;
* autorização por escopo;
* expiração ou rotação planejada de credenciais;
* revogação de réplica comprometida;
* auditoria de sincronização, anúncio de disponibilidade, falhas e mudanças de escopo.

Mecanismos aceitáveis devem ser baseados em soluções consolidadas, como:

* mTLS para autenticação mútua;
* tokens curtos emitidos pelo Origin;
* assinatura forte de requisições usando bibliotecas maduras;
* combinação de mTLS com autorização por escopos;
* assinatura de payloads sensíveis quando necessário.

Quando houver assinatura de requisições, ela deve incluir, no mínimo:

* método HTTP;
* caminho;
* hash do corpo, quando houver corpo;
* timestamp;
* nonce ou identificador único;
* identidade da réplica;
* escopo da operação.

A verificação deve usar comparação segura de assinaturas fornecida por biblioteca confiável.

Não deve haver comparação manual vulnerável a timing attack.

## Proteção contra replay

Operações sensíveis devem possuir proteção contra replay.

Exemplos de operações sensíveis:

* sincronização de fragmentos;
* anúncio de disponibilidade;
* emissão de pacote de acesso;
* revogação;
* alteração de política;
* registro de Replica/Edge;
* ações administrativas.

Controles recomendados:

* timestamps com janela curta;
* nonce único por requisição sensível;
* expiração curta de tickets e pacotes de acesso;
* rejeição de requisições repetidas;
* associação entre assinatura, método, rota e corpo;
* auditoria de tentativas rejeitadas.

O controle de replay deve ser implementado com bibliotecas, armazenamento e mecanismos confiáveis. Não deve depender apenas de validações frágeis no cliente.

## Autorização Origin <-> Replica/Edge

Uma réplica autenticada ainda precisa ser autorizada.

A autorização deve controlar:

* quais buckets a réplica pode sincronizar;
* quais objetos a réplica pode armazenar;
* quais fragmentos pode anunciar;
* quais fragmentos pode servir;
* quais regiões ou grupos operacionais pode atender;
* quais políticas se aplicam;
* por quanto tempo a autorização é válida;
* quando deve parar de servir conteúdo revogado.

Replica/Edge não deve:

* aceitar upload arbitrário de clientes;
* emitir autorização própria;
* decidir autonomamente que objetos pode distribuir;
* continuar servindo objeto revogado;
* compartilhar conteúdo fora do escopo recebido;
* ser tratada como confiável apenas por estar em infraestrutura própria.

## Ameaças principais

A arquitetura deve considerar, no mínimo, as seguintes ameaças:

* cliente tenta baixar objeto sem consultar o Origin;
* cliente tenta reutilizar pacote de acesso expirado;
* cliente tenta acessar objeto revogado;
* peer envia fragmento corrompido;
* peer envia fragmento de outro objeto;
* peer tenta se passar por fonte autorizada;
* Replica/Edge comprometida anuncia disponibilidade falsa;
* Replica/Edge antiga continua servindo objeto revogado;
* requisição de sincronização é repetida por replay;
* URL temporária ou ticket vaza;
* usuário tenta enumerar buckets, objetos ou fontes;
* aplicação tenta acessar objeto fora do escopo;
* downgrade de política permite obter conteúdo por fonte não autorizada;
* abuso de fallback sobrecarrega o Origin;
* abuso de ranges gera tráfego excessivo;
* dashboard ou MCP executa ação destrutiva sem permissão adequada;
* operador administrativo altera política crítica sem auditoria;
* logs expõem tokens, segredos ou URLs sensíveis;
* falhas de configuração liberam acesso indevido.

## Controles por artefato

### Manifesto

O manifesto orienta a obtenção e validação dos fragmentos.

Controles obrigatórios:

* deve ser emitido, assinado ou validado pelo Origin;
* deve estar associado a objeto, versão e política;
* deve conter fragmentos, tamanhos, intervalos de bytes e hashes;
* deve possuir validade ou estar associado a um pacote de acesso válido;
* deve permitir validação de integridade independentemente da fonte;
* não deve aceitar hashes enviados por peers como autoridade;
* não deve ser alterável por Replica/Edge ou peer;
* deve ser rejeitado se estiver expirado, revogado, incompatível ou inválido.

Assinaturas de manifesto devem usar bibliotecas criptográficas maduras.

### Pacote de acesso

O pacote de acesso representa autorização temporária para obtenção.

Controles obrigatórios:

* deve ter expiração curta;
* deve conter escopo;
* deve indicar fontes autorizadas, quando aplicável;
* deve ser não adivinhável;
* deve ser resistente a replay;
* deve estar associado a usuário, aplicação ou contexto autorizado;
* deve poder ser invalidado por revogação;
* deve ser rejeitado se estiver expirado, adulterado ou fora do escopo;
* deve evitar exposição de segredos permanentes.

O formato exato pode ser token opaco, JWT assinado, ticket assinado, URL temporária ou mecanismo equivalente. A escolha deve usar bibliotecas consolidadas.

### Fragmentos

Fragmentos são partes do objeto e podem vir de fontes diferentes.

Controles obrigatórios:

* validar hash antes de marcar como concluído;
* rejeitar fragmento inválido mesmo vindo de Replica/Edge;
* rejeitar fragmento com tamanho incorreto;
* rejeitar fragmento fora do intervalo esperado;
* rejeitar fragmento associado a manifesto incompatível;
* registrar fontes com falhas repetidas;
* abrir circuito para fontes instáveis ou suspeitas;
* preservar apenas fragmentos validados;
* descartar dados inválidos ou não retomáveis.

A validação de hash deve usar funções e bibliotecas confiáveis da plataforma, sem implementação manual.

### Fontes autorizadas

Fontes autorizadas podem ser Origin, Replica/Edge ou peers autorizados.

Controles obrigatórios:

* devem ser emitidas ou aprovadas pelo Origin;
* devem possuir escopo;
* devem possuir validade;
* devem respeitar política de bucket e objeto;
* devem ser removidas quando expiradas ou revogadas;
* devem ser ignoradas quando apresentarem falhas repetidas;
* devem ser auditáveis quando usadas em operações sensíveis.

Uma fonte autorizada não é automaticamente fonte confiável para conteúdo. O fragmento ainda precisa ser validado.

### Revogação

Revogação deve impedir novas autorizações e limitar a continuidade de acessos.

Controles obrigatórios:

* Origin deve parar de emitir novas autorizações;
* SDK deve revalidar acessos longos conforme política;
* Replica/Edge revogada deve ser removida das fontes elegíveis;
* credenciais revogadas devem ser rejeitadas;
* pacotes de acesso revogados devem ser rejeitados;
* objetos revogados não devem gerar novos pacotes de acesso;
* eventos de revogação devem ser auditados.

Deleção lógica não promete apagamento físico imediato de cópias transitórias já fora do Origin. Ela impede novas obtenções autorizadas e deve orientar SDKs e réplicas a interromper ou deixar de servir conteúdo conforme política.

## Segurança da API S3-like

A API S3-like deve preservar o modelo de segurança do Origin.

Controles esperados:

* autenticação para operações protegidas;
* autorização por bucket e objeto;
* validação de escopo por operação;
* suporte a URLs temporárias ou mecanismo equivalente;
* restrição de métodos e cabeçalhos aceitos;
* validação de `Range`;
* limites para tamanho de objeto;
* limites para quantidade de requisições;
* respostas consistentes sem vazar existência de objetos quando a política não permitir;
* auditoria de operações sensíveis;
* proteção contra enumeração de buckets e objetos.

A compatibilidade S3-like não deve enfraquecer os requisitos próprios do Ponte Mesh.

## Segurança da API Ponte Mesh

A API Ponte Mesh concentra operações mais sensíveis que a API S3-like.

Ela pode controlar manifestos, pacotes de acesso, fontes autorizadas, Replica/Edge, métricas, auditoria, políticas e revogações.

Controles esperados:

* autenticação obrigatória;
* autorização por escopo;
* proteção contra replay em operações sensíveis;
* validação forte de entrada;
* auditoria de ações administrativas;
* limitação de taxa;
* proteção contra abuso de fallback;
* proteção contra abuso de sincronização;
* separação entre operações administrativas e operações do SDK;
* negação por padrão quando a política for ausente ou ambígua.

## Segurança de Replica/Edge

Replica/Edge deve ser tratada como infraestrutura auxiliar, não como autoridade.

Controles obrigatórios:

* identidade própria por réplica;
* credencial própria por réplica;
* autenticação forte com o Origin;
* autorização por escopo;
* sincronização apenas de conteúdo autorizado;
* anúncio apenas de fragmentos autorizados;
* remoção imediata das fontes elegíveis quando revogada;
* auditoria de sincronização e serviço de fragmentos;
* aplicação de revogações recebidas;
* rejeição de comandos fora do escopo;
* isolamento do armazenamento local da réplica;
* proteção contra exposição de segredos locais.

Uma réplica comprometida não deve comprometer a integridade do objeto, pois fragmentos continuam sendo validados por hash conforme manifesto autorizado.

## Segurança de peers

Peers devem ser tratados como não confiáveis.

Controles obrigatórios:

* peer só pode participar quando autorizado por política;
* peer não deve emitir autorização;
* peer não deve ser autoridade sobre hash, manifesto ou disponibilidade;
* fragmentos vindos de peer devem ser validados;
* peer que enviar fragmentos inválidos deve ser penalizado;
* peer com falhas repetidas deve ser removido ou ignorado temporariamente;
* participação de peer deve expirar;
* compartilhamento temporário deve respeitar escopo do pacote de acesso.

## Segurança do SDK

O SDK não pertence necessariamente a este repositório, mas os contratos do servidor devem permitir comportamento seguro.

O SDK deve:

* consultar o Origin antes da obtenção;
* rejeitar manifesto inválido;
* rejeitar pacote de acesso expirado;
* validar fragmentos por hash;
* preservar apenas fragmentos validados;
* aplicar fallback conforme política;
* revalidar transferências longas quando exigido;
* ignorar fontes expiradas ou revogadas;
* não aceitar peers fora do pacote de acesso;
* não armazenar segredos permanentes sem proteção adequada;
* reportar falhas relevantes para métricas e auditoria.

## Segurança administrativa

Operações administrativas são sensíveis e devem possuir controle reforçado.

Exemplos:

* publicar objeto;
* remover objeto;
* revogar objeto;
* revogar usuário;
* revogar aplicação;
* revogar réplica;
* alterar política de bucket;
* alterar política de objeto;
* alterar política de fallback;
* registrar Replica/Edge;
* alterar escopos;
* consultar auditoria;
* consultar métricas sensíveis.

Controles esperados:

* autenticação forte;
* autorização por papel e escopo;
* auditoria obrigatória;
* confirmação para ações destrutivas;
* proteção contra CSRF quando houver interface web baseada em sessão;
* limitação de taxa;
* segregação entre operação administrativa e operação de cliente;
* logs sem segredos;
* trilha de auditoria imutável ou resistente a adulteração, quando possível.

## MCP e administração

MCP é uma interface administrativa e de automação sobre o plano de controle.

MCP não deve fazer parte do plano de dados e não deve participar diretamente da transferência de fragmentos.

Operações via MCP devem exigir:

* autenticação;
* autorização;
* escopo;
* auditoria;
* proteção contra ações destrutivas acidentais;
* validação de entrada;
* limites operacionais;
* rastreabilidade da ação executada.

MCP não deve permitir bypass das regras do Origin.

Se uma ação não seria permitida pela API administrativa normal, também não deve ser permitida via MCP.

## Proteção de segredos

Segredos não devem ser versionados.

Incluem-se como segredos:

* chaves privadas;
* chaves de assinatura;
* tokens administrativos;
* credenciais de réplica;
* credenciais de banco de dados;
* secrets de OAuth;
* certificados privados;
* URLs temporárias sensíveis;
* tickets de acesso;
* credenciais de integração.

Regras recomendadas:

* usar secret manager, vault ou mecanismo seguro da infraestrutura;
* permitir rotação;
* permitir revogação;
* separar segredos por ambiente;
* não registrar segredos em logs;
* não retornar segredos em respostas de erro;
* não expor segredos em métricas;
* não usar segredos de desenvolvimento em produção.

## Logs e auditoria

Logs operacionais e eventos de auditoria devem ser separados quando necessário.

Logs podem registrar informações de diagnóstico.

Auditoria deve registrar eventos sensíveis e decisões relevantes de segurança.

Eventos recomendados para auditoria:

* emissão de pacote de acesso;
* negação de pacote de acesso;
* expiração de pacote de acesso;
* revogação;
* publicação de objeto;
* remoção lógica;
* falha de autenticação;
* falha de autorização;
* registro de Replica/Edge;
* sincronização de Replica/Edge;
* anúncio de disponibilidade;
* fragmento inválido;
* fonte removida por circuit breaker;
* alteração de política;
* operação administrativa sensível;
* evento MCP.

Auditoria deve registrar:

* quem executou;
* quando executou;
* qual recurso foi afetado;
* qual ação foi solicitada;
* qual foi o resultado;
* qual política foi aplicada;
* identificador de correlação da requisição.

Não devem ser registrados:

* conteúdo dos objetos;
* tokens completos;
* chaves privadas;
* secrets;
* credenciais;
* assinaturas completas quando isso aumentar risco;
* URLs temporárias completas, salvo se houver mascaramento adequado.

## Validação de entrada

Todas as APIs devem validar entrada de forma rigorosa.

Devem ser validados:

* nomes de buckets;
* chaves de objetos;
* cabeçalhos;
* ranges;
* tamanhos;
* tipos de conteúdo;
* identificadores de réplica;
* identificadores de pacote de acesso;
* políticas;
* filtros de consulta;
* parâmetros administrativos.

A validação deve evitar:

* path traversal;
* injeção;
* enumeração indevida;
* ranges abusivos;
* estouro de tamanho;
* requisições ambíguas;
* desserialização insegura;
* bypass por cabeçalhos inesperados.

## Limites anti-abuso

O sistema deve prever limites para reduzir abuso operacional.

Controles possíveis:

* rate limit por usuário;
* rate limit por aplicação;
* rate limit por réplica;
* rate limit por IP, quando aplicável;
* limite de emissão de pacotes de acesso;
* limite de tentativas de autenticação;
* limite de ranges por objeto;
* limite de fragmentos simultâneos;
* limite de fallback por sessão;
* limite de sincronização por réplica;
* limite de anúncios de disponibilidade;
* limite de erros antes de abrir circuito;
* bloqueio temporário de fontes suspeitas.

Os limites devem evitar que fallback, ranges, sincronização ou emissão de pacotes sejam usados para sobrecarregar o Origin.

## Políticas de fallback seguro

Fallback é necessário para disponibilidade, mas também pode ser abusado.

Controles recomendados:

* limitar tentativas por fragmento;
* limitar fallback por sessão;
* registrar motivo do fallback;
* preservar fragmentos validados;
* evitar reinício completo sem necessidade;
* aplicar circuit breaker para fontes ruins;
* limitar concorrência contra o Origin;
* diferenciar falha legítima de comportamento suspeito;
* auditar padrões anormais de fallback.

O fallback para o Origin não deve permitir bypass de autorização. Mesmo no fallback, a operação deve continuar dentro do escopo autorizado.

## Integridade e armazenamento

O armazenamento deve preservar a integridade entre objeto, manifesto e fragmentos.

Regras recomendadas:

* objeto armazenado deve corresponder ao manifesto gerado;
* fragmentos devem possuir hashes registrados;
* metadados críticos devem ser protegidos contra alteração indevida;
* alterações de objeto devem gerar nova versão ou invalidar manifesto anterior;
* deleção lógica deve alterar estado de disponibilidade;
* objeto revogado não deve gerar novas autorizações;
* dados temporários inválidos devem ser descartados.

## Dependências de segurança

Dependências usadas para segurança devem ser escolhidas com cuidado.

Regras recomendadas:

* preferir bibliotecas maduras e mantidas;
* acompanhar vulnerabilidades conhecidas;
* evitar bibliotecas abandonadas;
* manter dependências atualizadas;
* usar SCA para verificar CVEs;
* gerar SBOM quando possível;
* revisar mudanças de versão em bibliotecas críticas;
* evitar copiar trechos criptográficos da internet;
* evitar implementar manualmente padrões de segurança.

## Decisões pendentes

As seguintes decisões devem ser fechadas antes da implementação final dos contratos sensíveis:

* mecanismo padrão de autenticação entre Origin e Replica/Edge;
* uso de mTLS, assinatura HMAC, assinatura assimétrica ou combinação;
* formato exato do ticket ou pacote de acesso;
* uso de token opaco, JWT assinado, ticket assinado ou URL temporária;
* modelo de rotação de credenciais;
* política de revalidação durante transferências longas;
* limites anti-abuso para fallback;
* limites anti-abuso para sincronização;
* formato de auditoria;
* mecanismo de armazenamento seguro de segredos;
* política de expiração de manifestos;
* política de expiração de fontes autorizadas.

## Síntese

A segurança do Ponte Mesh depende de manter o Origin como autoridade central e tratar todas as fontes auxiliares como potencialmente não confiáveis.

Peers e Replica/Edge podem ajudar no plano de dados, mas não devem controlar autorização, manifesto, revogação ou integridade.

Todo fragmento deve ser validado.

Toda obtenção deve começar com autorização do Origin.

Toda configuração insegura deve negar acesso.

E, principalmente, mecanismos sensíveis de segurança devem ser implementados com bibliotecas, protocolos e frameworks consolidados, evitando soluções caseiras que possam comprometer a arquitetura.
