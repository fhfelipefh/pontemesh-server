# Ponte Mesh Server

**Ponte Mesh Server** é o componente de servidor do **Ponte Mesh**, uma proposta open source para distribuição híbrida de objetos digitais com controle centralizado, entrega por fragmentos e fallback automático para o servidor de origem.

O projeto tem como objetivo combinar a simplicidade operacional de uma arquitetura cliente-servidor com a eficiência de fontes auxiliares de distribuição, como nós **Replica/Edge** e peers autorizados, mantendo o **Origin** como autoridade central do sistema.

Na arquitetura proposta, o **Origin** permanece responsável por ingestão, catálogo, autenticação, autorização, geração de manifestos, revogação, políticas de expiração, métricas e fallback.

O plano de dados, por sua vez, pode utilizar fontes auxiliares para distribuir fragmentos de conteúdo quando essa estratégia for segura, autorizada e tecnicamente vantajosa.

## Por que este projeto existe

Arquiteturas tradicionais cliente-servidor concentram todo o tráfego no servidor de origem. Essa abordagem simplifica o controle, a segurança e a previsibilidade operacional, mas pode elevar custos de banda, aumentar a carga sobre a infraestrutura central e dificultar a escalabilidade em cenários de múltiplos acessos simultâneos.

O Ponte Mesh busca oferecer um modelo intermediário: preservar o **Origin** como ponto central de controle e, ao mesmo tempo, permitir que a transferência dos dados seja parcialmente descentralizada por meio de fragmentos.

Dessa forma, o sistema pode reduzir a dependência exclusiva do servidor de origem quando houver fontes auxiliares confiáveis e autorizadas. Caso a distribuição híbrida não seja possível, apresente falhas ou não ofereça desempenho adequado, o fallback para o **Origin** garante a continuidade da obtenção do conteúdo.

## Componentes principais

### Origin

Servidor central da arquitetura.

É responsável por controlar a publicação, ingestão, armazenamento primário, catálogo de objetos, autenticação, autorização, geração de manifestos, revogação, políticas de expiração, métricas e fallback.

O Origin é a autoridade do sistema. Nenhuma obtenção de conteúdo deve ocorrer sem autorização prévia emitida por ele.

### Replica/Edge

Nó auxiliar com maior estabilidade operacional.

Seu papel é replicar conteúdos autorizados e auxiliar na entrega de fragmentos, reduzindo a dependência exclusiva do Origin e de peers comuns.

O Replica/Edge não substitui o Origin e não deve atuar como atalho de segurança. Sua comunicação com o Origin deve ser autenticada, autorizada, auditável e revogável.

### SDK

Camada de integração consumida pelas aplicações cliente.

O SDK abstrai a complexidade da distribuição híbrida, sendo responsável por consultar o Origin, interpretar manifestos, selecionar fontes, obter fragmentos, validar integridade, controlar progresso e executar fallback automático quando necessário.

### Client

Aplicação consumidora dos objetos digitais.

O Client utiliza o SDK para acessar conteúdos sem precisar lidar diretamente com a complexidade da arquitetura híbrida. Quando permitido pelas políticas do Origin, também pode colaborar temporariamente com fragmentos já obtidos.

## Princípios do projeto

* Toda obtenção de conteúdo deve começar com autorização do Origin.
* O Origin é a autoridade central sobre publicação, disponibilidade, autenticação, autorização e revogação.
* O P2P deve ser utilizado como mecanismo de aceleração, não como substituto do controle central.
* Nós Replica/Edge devem atuar como reforço de disponibilidade, não como fonte autônoma de autorização.
* Todo fragmento recebido de qualquer fonte deve ser validado por integridade antes de ser aceito.
* Fragmentos inválidos, incompletos ou não autorizados devem ser descartados.
* Revogação e expiração devem impedir novas autorizações de acesso.
* O fallback para o Origin deve preservar fragmentos já validados, evitando reiniciar a obtenção completa do objeto.
* A arquitetura deve manter comportamento previsível mesmo quando peers estiverem indisponíveis, instáveis ou atrás de NAT e firewalls.
* Sempre que possível, a API deve ser familiar para integrações inspiradas no modelo S3.

## Objetivos arquiteturais

O Ponte Mesh Server busca viabilizar uma arquitetura em que:

1. O controle permaneça centralizado no Origin.
2. A transferência de dados possa ocorrer de forma híbrida.
3. Objetos sejam distribuídos por fragmentos verificáveis.
4. O SDK oculte a complexidade da obtenção híbrida.
5. O sistema continue funcional mesmo sem P2P.
6. A revogação e a expiração sejam respeitadas pelo fluxo de autorização.
7. O fallback para o Origin seja parte fundamental do comportamento esperado.
8. A integração com aplicações existentes seja simples e próxima de modelos conhecidos de armazenamento de objetos.

## Documentação

A documentação pública e o site do projeto estão disponíveis no repositório:

<https://github.com/fhfelipefh/pontemesh-docs>
