# Requisitos

Este documento consolida os requisitos derivados da proposta arquitetural e do escopo do repositório `pontemesh-server`.

O servidor deve atender aos papéis **Origin** e **Replica/Edge**, preservando a separação entre plano de controle e plano de dados. Quando não há fontes auxiliares elegíveis, o Origin serve objetos diretamente com as mesmas regras de autenticação, autorização, manifesto, integridade, métricas e revogação.

## Requisitos funcionais

### Operação e papéis

* **RF01:** O servidor deve operar como **Origin** ou **Replica/Edge**.
* **RF02:** O papel operacional do servidor deve ser configurado explicitamente.
* **RF03:** O Origin deve servir objetos diretamente quando não houver fontes auxiliares elegíveis.
* **RF04:** O Origin deve preservar autorização, manifesto, integridade, métricas e revogação na entrega direta.
* **RF05:** O Replica/Edge deve operar somente como fonte auxiliar autorizada pelo Origin.
* **RF05-A:** O servidor deve informar sua versão no estado público de setup, e o painel deve exibi-la discretamente durante toda a configuração inicial.
* **RF05-B:** O setup deve distinguir listeners internos de endpoints públicos e não apresentar a porta interna do painel como se fosse a porta HTTPS externa.
* **RF05-C:** O Origin deve permitir configurar endpoints públicos distintos para o painel/API Ponte Mesh e para a API S3-compatible.

### API S3-like

* **RF06:** O Origin deve expor uma API S3-like mínima para operações fundamentais de buckets e objetos.
* **RF07:** A API S3-like deve permitir criar e listar buckets.
* **RF08:** A API S3-like deve permitir envio de objetos ao Origin.
* **RF09:** A API S3-like deve permitir listagem de objetos.
* **RF10:** A API S3-like deve permitir consulta de metadados por `HEAD`.
* **RF11:** A API S3-like deve permitir recuperação de objetos.
* **RF12:** A API S3-like deve permitir recuperação parcial por intervalo de bytes.
* **RF13:** A API S3-like deve permitir remoção lógica de objetos.
* **RF14:** A API S3-like deve permitir geração de URL temporária ou mecanismo equivalente.
* **RF15:** A API S3-like deve ser usada preferencialmente para operações base de objeto.
* **RF16:** Funcionalidades específicas do Ponte Mesh que não couberem naturalmente no modelo S3 devem ser expostas por APIs próprias.

### API Ponte Mesh

* **RF17:** O Origin deve expor APIs próprias do Ponte Mesh para manifestos, pacotes de acesso, políticas, métricas, auditoria, disponibilidade, revogação e Replica/Edge.
* **RF18:** A API Ponte Mesh deve permitir obter pacote de acesso para um objeto.
* **RF19:** A API Ponte Mesh deve permitir consultar manifesto autorizado.
* **RF20:** A API Ponte Mesh deve permitir consultar estado de disponibilidade de objetos e fragmentos.
* **RF21:** A API Ponte Mesh deve permitir configurar políticas específicas de distribuição híbrida.
* **RF22:** A API Ponte Mesh deve permitir configurar estratégias de fallback.
* **RF23:** A API Ponte Mesh deve permitir configurar políticas de priorização de fragmentos.
* **RF24:** A API Ponte Mesh deve permitir consultar métricas operacionais.
* **RF25:** A API Ponte Mesh deve permitir consultar eventos de auditoria.
* **RF26:** A API Ponte Mesh deve fornecer contratos estáveis para SDKs.

### Ingestão, catálogo e objetos

* **RF27:** O Origin deve permitir ingestão de objetos.
* **RF28:** O Origin deve manter catálogo de buckets, objetos, versões, metadados e estados de disponibilidade.
* **RF29:** O Origin deve armazenar ou referenciar o armazenamento primário dos objetos.
* **RF30:** O Origin deve registrar metadados essenciais de cada objeto, incluindo tamanho, tipo de conteúdo, versão, data de criação, estado e política aplicável.
* **RF31:** O Origin deve permitir deleção lógica de objetos.
* **RF32:** O Origin deve impedir novas obtenções de objetos removidos logicamente, expirados, revogados ou bloqueados.
* **RF33:** O Origin deve permitir recuperação direta de objetos quando não houver fontes auxiliares disponíveis.

### Manifestos e fragmentação

* **RF34:** O Origin deve gerar ou disponibilizar manifesto por objeto.
* **RF35:** O manifesto deve descrever a estrutura fragmentada do objeto.
* **RF36:** O manifesto deve conter, no mínimo, identificação do objeto, versão, lista de fragmentos, intervalos de bytes, tamanhos esperados e hashes de integridade.
* **RF37:** O manifesto deve conter informações suficientes para que o SDK valide fragmentos e reconstrua logicamente o objeto.
* **RF38:** O manifesto deve estar associado a uma autorização válida emitida pelo Origin.
* **RF39:** O Origin deve permitir que a obtenção por intervalo de bytes seja compatível com a estratégia de fallback por fragmento.

### Pacotes de acesso e autorização

* **RF40:** O Origin deve emitir pacote de acesso antes de qualquer obtenção controlada.
* **RF41:** O pacote de acesso deve conter autorização temporária.
* **RF42:** O pacote de acesso pode conter manifesto, credencial ou ticket temporário, prazo de expiração, fontes autorizadas, políticas de seleção e endpoints de fallback.
* **RF43:** O Origin deve negar obtenção sem autenticação válida.
* **RF44:** O Origin deve negar obtenção sem autorização válida.
* **RF45:** O Origin deve negar obtenção quando a política aplicável não permitir acesso.
* **RF46:** O Origin deve permitir uso de URLs temporárias, tickets ou credenciais não adivinháveis.
* **RF47:** O Origin deve permitir expiração de pacotes de acesso.
* **RF48:** O Origin deve permitir revogação de pacotes de acesso.
* **RF49:** O Origin deve permitir revalidação de autorização em transferências prolongadas.

### Revogação, expiração e deleção lógica

* **RF50:** O Origin deve permitir revogar acesso a objetos.
* **RF51:** O Origin deve permitir revogar acesso de usuários.
* **RF52:** O Origin deve permitir revogar acesso de aplicações.
* **RF53:** O Origin deve permitir revogar Replica/Edge.
* **RF54:** O Origin deve impedir novas autorizações para objetos revogados, expirados, bloqueados ou removidos logicamente.
* **RF55:** O Origin deve comunicar revogações relevantes a Replica/Edge.
* **RF56:** O SDK deve receber informações suficientes para interromper ou revalidar transferências quando houver expiração ou revogação.
* **RF57:** A deleção lógica não deve pressupor apagamento físico imediato de cópias transitórias já distribuídas.

### Fallback e recuperação parcial

* **RF58:** O Origin deve atuar como fonte direta e fonte final de garantia.
* **RF59:** O sistema deve permitir fallback para o Origin.
* **RF60:** O fallback deve ocorrer preferencialmente por fragmento ou intervalo de bytes.
* **RF61:** O fallback deve preservar fragmentos já validados.
* **RF62:** O sistema não deve exigir reinício completo da obtenção quando apenas alguns fragmentos falharem.
* **RF63:** O Origin deve permitir recuperação parcial por intervalo de bytes para apoiar retomada e fallback.
* **RF64:** O sistema deve registrar eventos de fallback para métricas e auditoria.
* **RF65:** O sistema deve permitir fallback total da sessão quando múltiplas falhas tornarem a distribuição auxiliar inviável.

### Replica/Edge

* **RF66:** O Replica/Edge deve possuir identidade própria.
* **RF67:** O Replica/Edge deve autenticar-se com o Origin.
* **RF68:** Toda comunicação entre Origin e Replica/Edge deve ser autenticada e autorizada.
* **RF69:** O Replica/Edge deve operar com escopos explícitos.
* **RF70:** O Replica/Edge deve sincronizar apenas conteúdos autorizados a partir do Origin.
* **RF71:** O Replica/Edge deve obter plano de sincronização autorizado.
* **RF72:** O Replica/Edge deve anunciar disponibilidade de objetos ou fragmentos autorizados.
* **RF73:** O Replica/Edge deve reportar métricas de saúde, disponibilidade e transferência.
* **RF74:** O Replica/Edge deve receber e aplicar revogações.
* **RF75:** O Replica/Edge deve respeitar mudanças de política emitidas pelo Origin.
* **RF76:** O Replica/Edge não deve aceitar upload arbitrário de clientes.
* **RF77:** O Replica/Edge não deve emitir autorização própria de acesso.
* **RF77-A:** O Origin deve emitir no plano de sincronização o conjunto de réplicas elegíveis e o líder determinístico por objeto quando houver Replica/Edge autorizada.
* **RF77-B:** O Replica/Edge pode servir dados já sincronizados em modo degradado quando o Origin estiver temporariamente indisponível, desde que o pacote de acesso e token já tenham sido revalidados pelo Origin e a réplica seja líder no último conjunto de eleição recebido.
* **RF78:** O Origin deve remover réplicas revogadas, expiradas ou inválidas da lista de fontes elegíveis.

### SDK e Client

* **RF79:** O servidor deve fornecer contratos estáveis para SDKs multiplataforma.
* **RF80:** O SDK deve consultar obrigatoriamente o Origin antes da obtenção controlada.
* **RF81:** O SDK deve receber pacote de acesso emitido pelo Origin.
* **RF82:** O SDK deve interpretar manifestos.
* **RF83:** O SDK deve selecionar fontes autorizadas conforme política aplicável.
* **RF84:** O SDK deve validar fragmentos por hash antes de aceitá-los.
* **RF85:** O SDK deve preservar mapa local de progresso.
* **RF86:** O SDK deve acionar fallback quando peers ou Replica/Edge falharem, expirarem ou não forem vantajosos.
* **RF87:** O Client deve consumir uma interface de alto nível, sem precisar lidar diretamente com descoberta de peers, validação de fragmentos ou fallback.
* **RF88:** Quando permitido pela política, o Client pode colaborar temporariamente com fragmentos por meio do SDK.

### Métricas e auditoria

* **RF89:** O Origin deve registrar métricas de bytes servidos pelo Origin.
* **RF90:** O Origin deve registrar métricas de bytes servidos por Replica/Edge.
* **RF91:** O Origin deve registrar métricas de bytes servidos por peers quando essas informações forem reportadas pelo SDK.
* **RF92:** O sistema deve permitir calcular redução de carga no Origin.
* **RF93:** O sistema deve registrar taxa de fallback.
* **RF94:** O sistema deve registrar tentativas por fragmento.
* **RF95:** O sistema deve registrar fragmentos invalidados por hash.
* **RF96:** O sistema deve registrar eventos de autenticação, autorização, emissão de pacote de acesso, revogação, replicação, fallback e falhas.
* **RF97:** O sistema deve auditar operações administrativas sensíveis.
* **RF98:** O sistema deve permitir integração administrativa por painel, API e MCP.
* **RF99:** Eventos MCP devem ser registrados para auditoria.

## Requisitos não funcionais

### Segurança

* **RNF01:** O sistema deve adotar segurança por padrão.
* **RNF02:** O sistema deve negar acesso quando faltar política explícita.
* **RNF03:** Configurações inseguras devem falhar fechadas.
* **RNF04:** Segredos, tokens, chaves e credenciais não devem ser registrados em logs.
* **RNF05:** Credenciais administrativas, de usuários, aplicações e réplicas devem ser separadas.
* **RNF06:** Chaves e credenciais devem permitir rotação e revogação.
* **RNF07:** Toda comunicação Origin e Replica/Edge deve ser autenticada, autorizada, auditável e revogável.
* **RNF08:** Peers não devem ser tratados como fontes confiáveis sem validação de integridade.

### Integridade

* **RNF09:** A integridade deve ser verificável por fragmento.
* **RNF10:** Fragmentos inválidos devem ser descartados.
* **RNF11:** O objeto final deve ser reconstruído apenas com fragmentos validados.
* **RNF12:** O manifesto deve permitir validação dos fragmentos independentemente da fonte.

### Disponibilidade e tolerância a falhas

* **RNF13:** O Origin deve continuar funcional mesmo sem peers ou Replica/Edge.
* **RNF14:** A arquitetura deve tolerar falhas parciais de download.
* **RNF15:** O sistema deve preservar fragmentos já validados.
* **RNF16:** O fallback deve evitar reinício desnecessário da obtenção completa.
* **RNF17:** O sistema deve lidar com peers instáveis, indisponíveis ou atrás de NAT e firewalls.
* **RNF18:** Replica/Edge deve reforçar disponibilidade, mas não deve ser dependência obrigatória para o funcionamento do Origin.
* **RNF18-A:** Liderança degradada de Replica/Edge deve preservar continuidade do plano de dados sem promover a réplica a autoridade de controle.

### Contratos e interoperabilidade

* **RNF19:** Os contratos devem ser estáveis para SDKs multiplataforma.
* **RNF20:** A API S3-like deve buscar interoperabilidade conceitual com clientes S3 existentes sempre que possível.
* **RNF21:** A API S3-like não deve ser distorcida para representar conceitos específicos do Ponte Mesh.
* **RNF22:** Funcionalidades específicas da arquitetura híbrida devem ser expostas por APIs próprias.
* **RNF23:** A configuração deve ser explícita por papel operacional, como `origin` ou `replica-edge`.

### Observabilidade

* **RNF24:** O sistema deve possuir observabilidade suficiente para avaliar redução de carga no Origin.
* **RNF25:** O sistema deve permitir medir bytes servidos por Origin, Replica/Edge e peers.
* **RNF26:** O sistema deve permitir medir taxa de fallback.
* **RNF27:** O sistema deve permitir auditar operações sensíveis.
* **RNF28:** O sistema deve permitir correlação entre operações, objetos, buckets, fontes e sessões de transferência.
* **RNF29:** Métricas não devem expor conteúdo dos objetos nem segredos.

### Manutenibilidade e evolução

* **RNF30:** A arquitetura deve manter separação clara entre plano de controle e plano de dados.
* **RNF31:** O código deve preservar separação entre domínio S3-like e domínio Ponte Mesh.
* **RNF32:** O projeto deve favorecer evolução futura de dashboard administrativo.
* **RNF33:** O projeto deve manter MCP como interface administrativa do plano de controle, sem participar do plano de dados.
* **RNF34:** O projeto deve permitir evolução futura do SDK sem quebrar contratos essenciais.
* **RNF35:** O projeto deve evitar acoplamento entre regras internas do Origin e detalhes específicos de um único ambiente de cliente.

## Fora de escopo inicial

Estão fora do escopo inicial deste repositório:

* criar um novo protocolo P2P;
* garantir que toda transferência use P2P;
* tornar peers obrigatórios para funcionamento do sistema;
* tornar Replica/Edge obrigatório para funcionamento do Origin;
* implementar o SDK dentro deste repositório, salvo decisão futura;
* implementar o dashboard administrativo completo no escopo inicial;
* garantir deleção física imediata em peers ou cópias transitórias já distribuídas;
* substituir CDNs tradicionais em todos os cenários;
* garantir conectividade P2P em todos os ambientes de rede;
* resolver completamente limitações de NAT, firewall e churn;
* implementar todos os clientes multiplataforma no repositório do servidor;
* forçar funcionalidades específicas do Ponte Mesh dentro da API S3-like quando não houver correspondência natural.

## Síntese

O `pontemesh-server` deve implementar o núcleo de controle da arquitetura Ponte Mesh.

O **Origin** deve ser a autoridade central, responsável por ingestão, catálogo, autenticação, autorização, manifestos, pacotes de acesso, revogação, métricas e fallback.

O **Replica/Edge** deve atuar como fonte auxiliar autorizada, sem emitir autorização própria e sem substituir o Origin.

A distribuição híbrida deve reduzir carga do Origin quando houver condições seguras e vantajosas, mas o Origin deve continuar plenamente funcional mesmo sem peers ou réplicas.
