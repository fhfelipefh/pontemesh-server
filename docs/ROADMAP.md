# Roadmap conceitual

Este documento apresenta um roadmap conceitual para evolução do `pontemesh-server`.

O roadmap não representa um cronograma fechado de implementação. Ele organiza as principais etapas técnicas necessárias para transformar a proposta arquitetural em um servidor funcional, preservando os princípios centrais do Ponte Mesh:

* Origin como autoridade central;
* API S3-like para operações fundamentais de buckets e objetos;
* API Ponte Mesh para recursos específicos da arquitetura híbrida;
* objetos organizados por manifestos e fragmentos;
* autorização prévia antes da obtenção;
* Replica/Edge como fonte auxiliar autorizada;
* SDK como camada de abstração para obtenção híbrida;
* fallback para o Origin como garantia de continuidade;
* métricas para avaliar redução de carga no Origin.

A ausência de peers ou réplicas não caracteriza um modo separado de operação. Nesses casos, o Origin continua funcionando normalmente, servindo objetos diretamente e mantendo autenticação, autorização, manifesto, integridade, métricas e revogação.

## Fase 1: Conhecimento, arquitetura e contratos

Objetivo: consolidar a base conceitual do projeto antes da implementação.

Entregas esperadas:

* consolidar a documentação arquitetural do projeto;
* definir os papéis operacionais Origin e Replica/Edge;
* remover ambiguidades sobre modos inexistentes ou desnecessários;
* consolidar o glossário do projeto;
* definir requisitos funcionais e não funcionais;
* fechar requisitos de segurança;
* definir princípios de configuração segura;
* definir limites do escopo inicial;
* definir responsabilidades do Origin, Replica/Edge, SDK e Client;
* definir a separação entre plano de controle e plano de dados;
* definir os contratos conceituais entre Origin, Replica/Edge e SDK;
* documentar decisões arquiteturais relevantes em ADRs, quando necessário.

Critérios de conclusão:

* a arquitetura deve estar documentada de forma clara;
* o Origin deve estar definido como autoridade central;
* Replica/Edge deve estar definido como fonte auxiliar, não como autoridade;
* o SDK deve estar definido como consumidor de contratos, não como parte obrigatória deste repositório;
* deve estar claro que P2P e Replica/Edge são otimizações, não dependências obrigatórias para funcionamento do Origin.

## Fase 2: Segurança e modelo de autorização

Objetivo: definir a base de segurança antes de expor operações sensíveis.

Entregas esperadas:

* definir autenticação para usuários, aplicações, SDKs e réplicas;
* definir autorização por escopos;
* definir expiração de pacotes de acesso;
* definir revogação de usuários, aplicações, objetos e réplicas;
* definir modelo de tickets, tokens ou URLs temporárias;
* definir assinatura de manifestos e pacotes de acesso;
* definir rotação de chaves e credenciais;
* definir política fail-closed;
* definir separação entre credenciais administrativas, credenciais de aplicação e credenciais de réplica;
* definir auditoria mínima para operações sensíveis;
* definir como o SDK deve revalidar autorização em transferências prolongadas.

Critérios de conclusão:

* nenhuma obtenção de conteúdo deve ser possível sem autorização do Origin;
* toda credencial sensível deve ter escopo e expiração;
* revogação e expiração devem impedir novas autorizações;
* configurações inseguras devem negar acesso por padrão.

## Fase 3: API S3-like mínima

Objetivo: implementar ou especificar o subconjunto S3-like para operações fundamentais de buckets e objetos.

Entregas esperadas:

* criar bucket;
* listar buckets;
* enviar objeto;
* listar objetos;
* consultar metadados por `HEAD`;
* recuperar objeto por `GET`;
* recuperar intervalo de bytes com `Range`;
* remover logicamente objeto;
* gerar URL temporária ou mecanismo equivalente;
* padronizar respostas, erros e códigos HTTP;
* definir limites do subconjunto S3-like suportado;
* documentar o que é compatível, parcialmente compatível ou fora do escopo S3-like.

Critérios de conclusão:

* operações fundamentais de objeto devem funcionar pelo endpoint Origin;
* uma aplicação deve conseguir usar o Origin como endpoint S3-like dentro do subconjunto suportado;
* funcionalidades específicas do Ponte Mesh não devem ser forçadas dentro da API S3-like.

## Fase 4: Origin mínimo

Objetivo: implementar o núcleo funcional do Origin.

Entregas esperadas:

* catálogo de buckets;
* catálogo de objetos;
* versionamento conceitual ou estrutura preparada para versões;
* ingestão de objetos;
* armazenamento primário;
* metadados de objetos;
* estado de disponibilidade;
* deleção lógica;
* recuperação direta pelo Origin;
* recuperação por intervalo de bytes;
* emissão de pacote de acesso básico;
* geração ou disponibilização de manifesto;
* expiração de pacotes de acesso;
* revogação básica de objeto;
* métricas básicas de tráfego;
* eventos básicos de auditoria.

Critérios de conclusão:

* o Origin deve conseguir receber, catalogar, autorizar e entregar objetos diretamente;
* o Origin deve continuar funcional mesmo sem peers ou Replica/Edge;
* a recuperação por range deve estar disponível para apoiar fallback e retomada parcial;
* métricas mínimas de bytes servidos pelo Origin devem ser coletadas.

## Fase 5: Manifesto, fragmentação e integridade

Objetivo: estruturar a obtenção de objetos por fragmentos verificáveis.

Entregas esperadas:

* definir formato inicial do manifesto;
* gerar lista de fragmentos por objeto;
* registrar índice, intervalo de bytes, tamanho esperado e hash de cada fragmento;
* permitir validação de fragmentos por hash;
* permitir validação do objeto completo, quando aplicável;
* associar manifesto a objeto, versão e política;
* associar manifesto a pacote de acesso autorizado;
* preparar contratos para reconstrução lógica pelo SDK;
* registrar falhas de integridade;
* registrar fragmentos invalidados por hash.

Critérios de conclusão:

* cada fragmento deve possuir informações suficientes para validação;
* o SDK deve conseguir reconstruir logicamente o objeto com base no manifesto;
* fragmentos inválidos devem ser detectáveis;
* a origem do fragmento não deve dispensar validação.

## Fase 6: API Ponte Mesh

Objetivo: expor os contratos específicos da arquitetura híbrida que não pertencem naturalmente ao modelo S3-like.

Entregas esperadas:

* endpoint para obter pacote de acesso;
* endpoint para consultar manifesto autorizado;
* endpoint para consultar estado de disponibilidade;
* endpoint para revogar objeto, usuário, aplicação ou pacote de acesso;
* endpoint para consultar fontes autorizadas;
* endpoint para consultar políticas aplicáveis;
* endpoint para consultar métricas;
* endpoint para consultar auditoria;
* contratos para SDKs;
* contratos administrativos para uso futuro por dashboard;
* documentação da separação entre API S3-like e API Ponte Mesh.

Critérios de conclusão:

* o SDK deve conseguir obter todas as informações necessárias para iniciar uma obtenção controlada;
* o dashboard futuro deve ter base de APIs para operação administrativa;
* recursos específicos do Ponte Mesh devem ficar em APIs próprias, sem distorcer o contrato S3-like.

## Fase 7: Replica/Edge

Objetivo: implementar o papel auxiliar de Replica/Edge como reforço de disponibilidade do plano de dados.

Entregas esperadas:

* identidade própria de réplica;
* autenticação da réplica com o Origin;
* autorização Origin para Replica/Edge;
* escopos explícitos de atuação;
* registro de réplicas autorizadas;
* plano de sincronização autorizado;
* sincronização seletiva de objetos ou fragmentos;
* armazenamento local de conteúdos autorizados;
* anúncio de disponibilidade de fragmentos;
* métricas de saúde;
* métricas de transferência;
* aplicação de revogações;
* remoção de réplicas revogadas das fontes elegíveis;
* auditoria de operações de réplica.

Critérios de conclusão:

* Replica/Edge deve sincronizar apenas conteúdos autorizados;
* Replica/Edge não deve aceitar upload arbitrário de clientes;
* Replica/Edge não deve emitir autorização própria;
* Replica/Edge deve ser revogável pelo Origin;
* Origin deve continuar funcionando mesmo sem réplicas disponíveis.

## Fase 8: Contratos para SDK e obtenção híbrida

Objetivo: preparar os contratos necessários para que SDKs externos implementem obtenção híbrida.

Entregas esperadas:

* contrato de pacote de acesso;
* contrato de manifesto;
* contrato de fontes autorizadas;
* contrato de políticas de seleção de fragmentos;
* contrato de políticas de seleção de fontes;
* contrato de fallback;
* contrato de reporte de métricas pelo SDK;
* contrato para revalidação de autorização;
* contrato para anúncio temporário de fragmentos por clientes, quando permitido;
* documentação de estados do mapa de progresso;
* documentação de erros esperados.

Critérios de conclusão:

* o SDK deve conseguir consultar o Origin antes de qualquer obtenção;
* o SDK deve conseguir interpretar manifesto e fontes autorizadas;
* o SDK deve conseguir validar fragmentos;
* o SDK deve conseguir reportar métricas relevantes;
* o SDK deve conseguir operar diretamente com o Origin quando não houver peers ou Replica/Edge.

## Fase 9: Estratégias de distribuição híbrida

Objetivo: evoluir a lógica conceitual para uso de fontes auxiliares quando houver benefício técnico.

Entregas esperadas:

* seleção de fontes entre Origin, Replica/Edge e peers autorizados;
* suporte a políticas como `headers-first`, `priority-first` e `rarest-first`, quando aplicável;
* priorização de fragmentos iniciais;
* priorização de fragmentos raros;
* seleção por latência, vazão, taxa de sucesso e falhas recentes;
* circuit breaker de fontes instáveis;
* fallback por fragmento;
* fallback total de sessão em caso de falhas generalizadas;
* preservação de fragmentos já validados;
* revalidação de expiração e revogação em transferências longas;
* métricas detalhadas por fonte e por fragmento.

Critérios de conclusão:

* o P2P deve ser usado apenas quando autorizado e vantajoso;
* Replica/Edge deve ser usado como fonte auxiliar estável;
* Origin deve permanecer como fonte final de garantia;
* falhas em fontes auxiliares não devem reiniciar a obtenção completa do objeto;
* fragmentos validados devem ser preservados.

## Fase 10: Métricas, auditoria e observabilidade

Objetivo: medir o comportamento real da arquitetura e permitir avaliação da redução de carga no Origin.

Entregas esperadas:

* bytes servidos pelo Origin;
* bytes servidos por Replica/Edge;
* bytes servidos por peers;
* tempo total de download;
* tempo até primeiro uso;
* taxa de fallback;
* tentativas por fragmento;
* fragmentos invalidados por hash;
* falhas de autenticação;
* falhas de autorização;
* revogações emitidas e aplicadas;
* métricas por réplica;
* métricas por objeto;
* métricas por bucket;
* eventos administrativos sensíveis;
* eventos MCP, quando existir;
* exportação futura para ferramentas de observabilidade.

Métrica principal:

```text id="sfg8c5"
ReducaoOrigin = 1 - (BytesOriginHibrido / BytesOriginBase)
```

Critérios de conclusão:

* deve ser possível comparar cenário base cliente-servidor com cenário híbrido;
* deve ser possível calcular redução de carga no Origin;
* deve ser possível auditar decisões relevantes de autorização, revogação, replicação e fallback;
* métricas não devem expor segredos nem conteúdo dos objetos.

## Fase 11: Avaliação experimental

Objetivo: validar tecnicamente a proposta em cenários controlados.

Cenários esperados:

* cenário base cliente-servidor, com entrega direta pelo Origin;
* cenário com SDK e Origin, sem fontes auxiliares;
* cenário com Origin e Replica/Edge;
* cenário com Origin, SDK e peers autorizados;
* cenário híbrido com Origin, Replica/Edge e peers;
* cenário com falhas controladas em peers;
* cenário com falhas controladas em Replica/Edge;
* cenário com revogação durante transferência;
* cenário com expiração de pacote de acesso;
* cenário com fragmentos inválidos;
* cenário com baixa disponibilidade de fontes auxiliares;
* cenário com fallback por fragmento;
* cenário com fallback total de sessão.

Métricas mínimas para avaliação:

* redução proporcional de bytes servidos pelo Origin;
* tempo total de download;
* tempo até primeiro uso;
* vazão média por cliente;
* taxa de fallback;
* bytes por fonte;
* fragmentos invalidados por hash;
* tentativas por fragmento;
* disponibilidade percebida pelo cliente.

Critérios de conclusão:

* deve ser possível demonstrar quando a distribuição híbrida reduz carga no Origin;
* deve ser possível demonstrar que o sistema continua funcional sem peers ou réplicas;
* deve ser possível demonstrar que fallback preserva progresso validado;
* deve ser possível demonstrar que revogação e expiração impedem novas autorizações.

## Fase 12: Operação administrativa e dashboard futuro

Objetivo: preparar o servidor para operação prática e futura interface administrativa.

Entregas esperadas:

* APIs administrativas para políticas;
* APIs administrativas para buckets e objetos;
* APIs administrativas para Replica/Edge;
* APIs administrativas para métricas;
* APIs administrativas para auditoria;
* APIs administrativas para revogação;
* APIs para configuração de estratégias de fallback;
* APIs para configuração de priorização de fragmentos;
* contratos para dashboard;
* integração MCP futura, quando aplicável.

Critérios de conclusão:

* deve existir base contratual para um dashboard administrativo;
* configurações específicas do Ponte Mesh devem estar disponíveis por APIs próprias;
* a API S3-like deve permanecer focada em operações fundamentais de objeto.

## Síntese

O roadmap prioriza primeiro a consolidação da arquitetura, segurança e contratos.

Em seguida, evolui para o **Origin mínimo**, depois para manifestos, fragmentação, API Ponte Mesh, Replica/Edge, contratos de SDK, distribuição híbrida, métricas e avaliação experimental.

O objetivo final é demonstrar uma arquitetura em que o Origin mantém controle centralizado e continua funcional sozinho, enquanto Replica/Edge e peers autorizados podem reduzir sua carga quando houver condições seguras e tecnicamente vantajosas.
