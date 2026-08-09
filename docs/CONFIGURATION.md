# Configuração

Este documento descreve a configuração conceitual do `pontemesh-server`.

Este documento serve como referência arquitetural para a configuração do servidor, das réplicas e das políticas operacionais.

O `pontemesh-server` é uma aplicação única. O papel operacional da instância é definido durante o setup ou pelas configurações administrativas. A mesma aplicação pode operar como **Origin** ou **Replica/Edge** de acordo com a configuração persistida.

## Blocos de configuração

A configuração deve ser organizada conforme o papel operacional persistido para a instância.

Os papéis previstos são:

* **Origin**;
* **Replica/Edge**.

Quando não há fontes auxiliares elegíveis, o Origin atende diretamente às operações autorizadas.

## Origin

O bloco de configuração do **Origin** deve concentrar os parâmetros relacionados ao servidor central da arquitetura.

Configurações esperadas:

* endereço de escuta;
* URL pública web usada em contratos entregues a clientes, quando diferente do endereço interno;
* URL pública S3 usada em manifestos, fontes e fallback, quando diferente do endereço interno;
* armazenamento primário;
* PostgreSQL;
* chave de assinatura de manifestos;
* chave de assinatura de pacotes de acesso;
* políticas de bucket;
* políticas de objeto;
* expiração padrão de pacotes de acesso;
* limites para recuperação por intervalo de bytes;
* limites e regras de fallback;
* credenciais administrativas;
* configuração da API S3-like;
* configuração das APIs próprias do Ponte Mesh;
* registro de réplicas autorizadas;
* políticas de autorização para Replica/Edge;
* políticas de distribuição por peers autorizados;
* métricas;
* auditoria;
* parâmetros de revogação e expiração.

O Origin deve ser configurado como autoridade central do sistema. Ele é responsável por autenticar, autorizar, emitir pacotes de acesso, gerar manifestos, controlar disponibilidade e aplicar políticas de revogação.

O Origin atende às operações fundamentais de objeto e coordena a distribuição híbrida quando houver política aplicável.

Em ambientes com Docker, proxy reverso ou múltiplas instâncias, `PONTEMESH_PUBLIC_WEB_URL` e `PONTEMESH_PUBLIC_S3_URL` podem ser definidos para que access packages, fontes autorizadas e fallback retornem endpoints alcançáveis pelo cliente externo, em vez de nomes internos da rede de containers.

O painel e a API S3-compatible usam listeners internos separados. Por padrão,
o painel escuta em `8080` e o S3-compatible em `9000`. Essas não são
necessariamente as portas públicas: um proxy pode publicar, por exemplo, o
painel em `https://origin.example.com` e o S3-compatible em
`https://s3.example.com`, ambos com TLS na porta `443`, ou publicar o segundo em
uma porta TLS dedicada como `9443`.

O setup persiste esses valores nas seções `[http]`, `[s3]` e `[public]` do
arquivo da instância. Para implantações gerenciadas, variáveis de ambiente têm
precedência sobre os valores persistidos:

```text
PONTEMESH_HTTP_HOST=127.0.0.1
PONTEMESH_WEB_PORT=8080
PONTEMESH_S3_PORT=9000
PONTEMESH_PUBLIC_WEB_URL=https://origin.example.com
PONTEMESH_PUBLIC_S3_URL=https://s3.example.com
```

Não é necessário nem recomendado expor diretamente `8080` ou `9000` quando um
proxy reverso local encaminha os endpoints públicos.

Ao usar um IP como endpoint S3 público, configure clientes S3-compatible para o
estilo de URL path. O estilo virtual-host transforma o bucket em subdomínio e,
por exemplo, tentaria resolver `bucket.134.65.234.41`, o que não é um hostname
DNS válido. Um domínio com DNS wildcard e certificado correspondente pode usar
virtual-host style; isso não é obrigatório para redes locais ou instalações por
IP.

## Replica/Edge

O bloco de configuração de **Replica/Edge** deve concentrar os parâmetros necessários para que uma réplica autorizada se comunique com o Origin e participe do plano de dados.

Configurações esperadas:

* identidade da réplica;
* endpoint do Origin;
* credencial, certificado ou chave da réplica;
* escopos permitidos;
* armazenamento local;
* política de sincronização;
* limites de banda;
* limites de armazenamento;
* intervalo de anúncio de disponibilidade;
* política de retenção local;
* comportamento ao receber revogação;
* comportamento ao receber mudanças de política;
* métricas de saúde;
* métricas de transferência;
* parâmetros de reconexão com o Origin.

Replica/Edge opera dentro dos escopos, políticas e autorizações definidos pelo Origin.

Toda comunicação entre Origin e Replica/Edge deve ser autenticada, autorizada, auditável e revogável.

## Políticas de distribuição

As configurações do Ponte Mesh podem incluir políticas específicas para controlar quando e como a distribuição híbrida será utilizada.

Essas políticas podem ser aplicadas em diferentes níveis, como:

* global;
* bucket;
* objeto;
* aplicação;
* usuário;
* réplica.

Exemplos de configurações possíveis:

* habilitar ou desabilitar distribuição por peers;
* habilitar ou desabilitar uso de Replica/Edge;
* definir prioridade entre Origin, Replica/Edge e peers;
* configurar limites de falha antes de acionar fallback;
* configurar expiração de pacotes de acesso;
* definir se a obtenção deve priorizar fragmentos iniciais;
* definir se a obtenção deve priorizar fragmentos raros;
* configurar estratégias como `headers-first`, `priority-first`, `rarest-first` ou políticas equivalentes;
* configurar pesos para seleção de fontes;
* configurar limites mínimos de vazão;
* configurar limites máximos de latência;
* configurar limites para circuit breaker de fontes;
* configurar revalidação de autorização durante transferências prolongadas.

Essas políticas pertencem ao domínio específico do Ponte Mesh.

## Segurança de configuração

A configuração deve seguir princípios de segurança desde o início do projeto.

Regras obrigatórias:

* segredos ficam fora do versionamento;
* credenciais administrativas não devem ser reutilizadas por réplicas;
* credenciais de réplica devem ser separadas de credenciais de usuários e aplicações;
* chaves de assinatura devem permitir rotação;
* revogação de credenciais deve ser prevista desde o início;
* configurações inseguras devem falhar fechadas, negando acesso;
* permissões devem ser explícitas;
* ausência de configuração obrigatória deve impedir inicialização segura;
* logs mascaram segredos, tokens, chaves e credenciais;
* configurações de produção usam valores explícitos.

## Configuração operacional

O papel operacional da instância deve ser explícito e carregado em runtime.

Valores conceituais possíveis:

* `origin`;
* `replica-edge`.

Quando nenhuma fonte auxiliar estiver disponível, o Origin atende diretamente às operações autorizadas, preservando segurança, manifesto, integridade e auditoria.

## Síntese

A configuração do Ponte Mesh deve separar claramente os parâmetros do **Origin** e do **Replica/Edge**.

O **Origin** é sempre a autoridade central.

O **Replica/Edge** é um componente auxiliar, autorizado pelo Origin, usado para reforçar disponibilidade e reduzir carga em cenários apropriados.

A distribuição híbrida é controlada por políticas explícitas.
