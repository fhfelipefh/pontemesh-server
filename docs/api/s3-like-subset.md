# Subconjunto S3-like

A proposta utiliza o modelo S3 como inspiração prática para reduzir a barreira de adoção do Ponte Mesh.

O objetivo é permitir que aplicações já familiarizadas com armazenamento de objetos possam interagir com o **Origin** por meio de operações conhecidas, como criação de buckets, envio de objetos, consulta de metadados, leitura, recuperação por intervalo de bytes e remoção lógica.

A API S3-like é o contrato principal para operações fundamentais de buckets e objetos. Manifestos, pacotes de acesso, políticas de fragmentação, Replica/Edge, seleção de fontes, fallback, métricas e auditoria ficam em APIs próprias.

## Objetivo

O objetivo do subconjunto S3-like é permitir que aplicações existentes, dentro do escopo suportado, consigam trocar principalmente o endpoint de armazenamento tradicional por um endpoint **Origin** do Ponte Mesh.

Exemplo conceitual:

```text
endpoint = https://s3.amazonaws.com
```

poderia ser substituído por:

```text
endpoint = https://origin-s3.exemplo.com
```

Desde que a aplicação utilize apenas o subconjunto suportado, a lógica principal de envio, leitura e consulta de objetos deve permanecer familiar.

A diferença está no comportamento interno do Origin e do SDK. Enquanto uma API S3 tradicional entrega o objeto diretamente a partir do serviço de armazenamento, o Ponte Mesh pode aplicar autorização, manifesto, fragmentação, validação de integridade, seleção de fontes e fallback.

## Operações esperadas

O subconjunto S3-like deve contemplar, no mínimo:

* Create Bucket;
* List Buckets;
* PUT Object;
* List Objects;
* HEAD Object;
* GET Object;
* GET Object com `Range`;
* DELETE Object como deleção lógica;
* URL temporária ou mecanismo equivalente.

Na implementação atual, o painel web/admin e a API S3-compatible usam endpoints
separados. O painel usa `http://localhost:8080`; o endpoint S3-compatible usa
`http://localhost:9000`:

```http
GET /
PUT /{bucket}
GET /{bucket}?list-type=2
HEAD /{bucket}
DELETE /{bucket}
PUT /{bucket}/{objectKey}
HEAD /{bucket}/{objectKey}
GET /{bucket}/{objectKey}
DELETE /{bucket}/{objectKey}
```

Todas exigem credenciais S3 próprias e AWS Signature Version 4.

O endpoint S3-compatible também aceita URLs pré-assinadas SigV4 por query
string (`X-Amz-*`) para acesso temporário. A validação continua usando a chave
S3 gerenciada no catálogo, respeita revogação da chave e rejeita URLs expiradas.

Essas operações representam o núcleo de armazenamento e recuperação de objetos.

## Create Bucket

Operação responsável por criar um bucket lógico no Origin.

O bucket deve funcionar como contêiner de objetos, seguindo a inspiração do modelo S3.

A criação de bucket deve respeitar autenticação, autorização, política administrativa e validação de nomes.

## List Buckets

Operação responsável por listar buckets visíveis para a entidade autenticada.

A listagem respeita escopos e políticas de visibilidade.

## PUT Object

Operação responsável por enviar um objeto ao Origin.

Ao receber um objeto, o Origin deve:

* validar autenticação e autorização;
* armazenar o conteúdo primário;
* registrar o objeto no catálogo;
* registrar metadados;
* associar o objeto ao bucket;
* preparar informações para manifesto e fragmentação;
* aplicar política de bucket ou objeto;
* registrar métricas e eventos de auditoria quando aplicável.

O envio de objeto ocorre pelo Origin.

## List Objects

Operação responsável por listar objetos dentro de um bucket.

A listagem deve respeitar autenticação, autorização, paginação, filtros, prefixos e políticas de visibilidade.

A resposta pode incluir informações como:

* chave do objeto;
* tamanho;
* versão;
* data de criação;
* data de modificação;
* estado de disponibilidade;
* metadados básicos.

## HEAD Object

Operação responsável por consultar metadados de um objeto sem transferir seu conteúdo.

Pode retornar informações como:

* tamanho do objeto;
* tipo de conteúdo;
* versão;
* hash ou identificador de integridade, quando aplicável;
* data de criação;
* data de modificação;
* estado de disponibilidade;
* metadados customizados;
* suporte a recuperação por intervalo de bytes.

Essa operação é importante para clientes que precisam validar existência, tamanho ou estado de um objeto antes da recuperação.

## GET Object

Operação responsável por recuperar um objeto.

Externamente, o `GET Object` deve manter comportamento familiar para aplicações que usam clientes S3 dentro do subconjunto suportado.

Internamente, o Origin pode aplicar regras próprias da arquitetura Ponte Mesh, como:

* autorização prévia;
* geração ou consulta de pacote de acesso;
* manifesto;
* fragmentação;
* validação de disponibilidade;
* registro de métricas;
* uso de SDK para obtenção híbrida;
* fallback para o Origin.

Quando a chamada for feita diretamente contra a API S3-like do Origin, o Origin deve conseguir atender o objeto diretamente, desde que a requisição esteja autorizada.

Quando a obtenção envolver SDK, o SDK pode usar manifesto e pacote de acesso para obter fragmentos de Origin, Replica/Edge ou peers autorizados, conforme política aplicável.

## GET Object com Range

Operação responsável por recuperar um intervalo específico de bytes de um objeto.

Essa operação é essencial para:

* retomada parcial de downloads;
* leitura progressiva;
* obtenção de fragmentos específicos;
* fallback por fragmento;
* preservação de progresso validado;
* alternância entre fontes sem reiniciar o objeto completo.

O suporte a `Range` deve respeitar limites operacionais para evitar abuso e sobrecarga do Origin.

Ranges inválidos devem ser rejeitados com resposta apropriada.

## DELETE Object

Operação responsável por remover logicamente um objeto.

No Ponte Mesh, `DELETE Object` deve ser tratado como deleção lógica ou alteração de estado de disponibilidade, conforme política aplicável.

A deleção lógica deve impedir novas autorizações de obtenção pelo Origin.

Após a deleção lógica, o Origin deve:

* impedir novos pacotes de acesso;
* atualizar o estado do objeto;
* registrar evento de auditoria;
* comunicar revogações a Replica/Edge quando aplicável;
* impedir que fontes revogadas continuem sendo elegíveis.

## URL temporária ou mecanismo equivalente

A API deve permitir algum mecanismo de acesso temporário.

Esse mecanismo pode ser:

* URL temporária;
* URL assinada;
* ticket temporário;
* token opaco;
* pacote de acesso;
* outro mecanismo equivalente.

Independentemente do formato, o mecanismo deve possuir:

* escopo;
* expiração;
* imprevisibilidade;
* possibilidade de revogação;
* proteção contra replay quando necessário;
* vínculo com política aplicável.

A implementação deve usar bibliotecas e mecanismos consolidados para assinatura, geração de tokens, validação criptográfica e comparação segura.

## Compatibilidade

O objetivo da compatibilidade S3-like é facilitar adoção com um subconjunto prático do Amazon S3.

A compatibilidade deve ser entendida como um subconjunto prático e documentado.

Aplicações já baseadas em clientes S3 devem conseguir trocar principalmente o endpoint quando utilizarem apenas as operações suportadas.

Recursos avançados podem ser documentados conforme entrarem no escopo.

## Diferença interna

A principal diferença entre uma API S3 convencional e o Ponte Mesh está no comportamento interno.

Em um serviço S3 tradicional, o objeto normalmente é entregue diretamente pela infraestrutura de armazenamento.

No Ponte Mesh, a operação externa pode parecer semelhante, mas internamente o sistema pode envolver:

* Origin como autoridade central;
* autorização temporária;
* manifesto;
* fragmentação;
* validação de integridade;
* seleção de fontes;
* Replica/Edge;
* peers autorizados;
* fallback para o Origin;
* métricas de bytes por fonte;
* auditoria de acesso e revogação.

Assim, a API S3-like reduz a barreira de integração, enquanto a API Ponte Mesh permite controlar os comportamentos específicos da distribuição híbrida.

## Recursos da API Ponte Mesh

Recursos específicos da arquitetura híbrida pertencem à API Ponte Mesh:

* emissão de pacote de acesso;
* consulta de manifesto autorizado;
* consulta de fontes autorizadas;
* configuração de Replica/Edge;
* anúncio de disponibilidade de fragmentos;
* configuração de políticas de fallback;
* configuração de estratégias como `headers-first`, `priority-first` ou `rarest-first`;
* métricas por fonte;
* auditoria operacional;
* revogação de réplica;
* revalidação de autorização durante transferências longas;
* controle de peers autorizados;
* políticas específicas de distribuição híbrida.

## Relação com a API Ponte Mesh

A API S3-like deve responder pelas operações fundamentais de armazenamento de objetos.

A API Ponte Mesh deve responder pelos recursos específicos da arquitetura híbrida.

Em resumo:

* S3-like cuida de buckets, objetos, metadados, envio, leitura, range e deleção lógica;
* Ponte Mesh cuida de manifesto, pacote de acesso, fontes autorizadas, fallback, Replica/Edge, políticas, métricas, auditoria e contratos para SDKs.

As duas APIs são complementares.

## Segurança

Todas as operações S3-like devem respeitar o modelo de segurança do Origin.

Regras obrigatórias:

* autenticação quando exigida pela política;
* autorização por bucket e objeto;
* autorização explícita;
* validação de escopo;
* expiração de acessos temporários;
* auditoria de operações sensíveis;
* proteção contra enumeração indevida;
* validação de nomes de buckets e chaves de objetos;
* validação de ranges;
* limites de tamanho e taxa quando necessário;
* respostas de erro sem segredos ou detalhes internos sensíveis.

A compatibilidade S3-like preserva o modelo de segurança da arquitetura.

## Síntese

O subconjunto S3-like existe para tornar o Ponte Mesh mais fácil de adotar.

Ele deve cobrir as operações essenciais de buckets e objetos, permitindo integração familiar para aplicações já acostumadas com armazenamento de objetos.

A arquitetura interna, porém, continua sendo própria do Ponte Mesh: Origin como autoridade central, objetos fragmentados, manifestos, autorização temporária, validação de integridade, Replica/Edge, peers autorizados e fallback para o Origin.

O S3-like é a porta de entrada para operações comuns. A API Ponte Mesh é o espaço correto para os recursos avançados da distribuição híbrida.
