# Ponte Mesh Server

**Ponte Mesh Server** é o componente de servidor do **Ponte Mesh**, uma proposta open source para distribuição híbrida de objetos digitais com controle centralizado, entrega por fragmentos e fallback automático para o servidor de origem.

O projeto tem como objetivo combinar a simplicidade operacional de uma arquitetura cliente-servidor com a eficiência de fontes auxiliares de distribuição, como nós **Replica/Edge** e peers autorizados, mantendo o **Origin** como autoridade central do sistema.

Na arquitetura proposta, o **Origin** permanece responsável por ingestão, catálogo, autenticação, autorização, geração de manifestos, revogação, políticas de expiração, métricas e fallback.

O plano de dados, por sua vez, pode utilizar fontes auxiliares para distribuir fragmentos de conteúdo quando essa estratégia for segura, autorizada e tecnicamente vantajosa.

## Por que este projeto existe

Arquiteturas tradicionais cliente-servidor concentram todo o tráfego no servidor de origem. Essa abordagem simplifica o controle, a segurança e a previsibilidade operacional, mas pode elevar custos de banda, aumentar a carga sobre a infraestrutura central e dificultar a escalabilidade em cenários de múltiplos acessos simultâneos.

O Ponte Mesh busca oferecer um modelo intermediário: preservar o **Origin** como ponto central de controle e, ao mesmo tempo, permitir que a transferência dos dados seja parcialmente descentralizada por meio de fragmentos.

Dessa forma, o sistema reduz a dependência exclusiva do servidor de origem quando houver fontes auxiliares confiáveis e autorizadas. O **Origin** garante a continuidade da obtenção quando a entrega auxiliar não for aplicável.

## Componentes principais

### Origin

Servidor central da arquitetura.

É responsável por controlar a publicação, ingestão, armazenamento primário, catálogo de objetos, autenticação, autorização, geração de manifestos, revogação, políticas de expiração, métricas e fallback.

O Origin é a autoridade do sistema. Nenhuma obtenção de conteúdo deve ocorrer sem autorização prévia emitida por ele.

### Replica/Edge

Nó auxiliar com maior estabilidade operacional.

Seu papel é replicar conteúdos autorizados e auxiliar na entrega de fragmentos, reduzindo a dependência exclusiva do Origin e de peers comuns.

O Replica/Edge opera sob autorização do Origin, com comunicação autenticada, auditável e revogável.

### SDK

Camada de integração consumida pelas aplicações cliente.

O SDK abstrai a complexidade da distribuição híbrida, sendo responsável por consultar o Origin, interpretar manifestos, selecionar fontes, obter fragmentos, validar integridade, controlar progresso e executar fallback automático quando necessário.

### Client

Aplicação consumidora dos objetos digitais.

O Client utiliza o SDK para acessar conteúdos sem precisar lidar diretamente com a complexidade da arquitetura híbrida. Quando permitido pelas políticas do Origin, também pode colaborar temporariamente com fragmentos já obtidos.

## Princípios do projeto

* Toda obtenção de conteúdo deve começar com autorização do Origin.
* O Origin é a autoridade central sobre publicação, disponibilidade, autenticação, autorização e revogação.
* O P2P é mecanismo de aceleração subordinado ao controle central.
* Nós Replica/Edge reforçam disponibilidade dentro do escopo autorizado pelo Origin.
* Todo fragmento recebido de qualquer fonte deve ser validado por integridade antes de ser aceito.
* Fragmentos aceitos precisam corresponder ao manifesto autorizado.
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

## Construção

Para construir o projeto localmente, primeiro gere o build do painel web:

```bash
cd web
npm install
npm run build
cd ..
```

Depois compile o servidor Rust:

```bash
cargo build
```

O servidor exige PostgreSQL. Defina obrigatoriamente:

```text
PONTEMESH_DATABASE_URL=postgres://pontemesh:pontemesh@postgres:5432/pontemesh
```

Em Docker, `postgres` é o nome do serviço na rede dedicada. Em execução direta
fora do Docker, substitua o host da URL pelo endereço do PostgreSQL acessível ao
processo local.

O servidor usa PostgreSQL como banco da aplicação e falha na inicialização se a conexão estiver indisponível.

Para gerar o binário otimizado:

```bash
cargo build --release
```

O executável será gerado em:

```text
target/release/pontemesh-server
```

## Execução

Para executar em ambiente local:

```bash
cargo run
```

Ou, após o build em modo release:

```bash
./target/release/pontemesh-server
```

Por padrão, o servidor utiliza:

```text
PONTEMESH_HOME=/var/pontemesh_home
PONTEMESH_DATABASE_URL=<obrigatório>
PONTEMESH_STORAGE_PATH=/var/pontemesh_home/data/storage
PONTEMESH_HTTP_HOST=0.0.0.0
PONTEMESH_WEB_PORT=8080
PONTEMESH_S3_PORT=9000
```

O diretório persistente da instância é `PONTEMESH_HOME`. Em containers, monte um
volume para `/var/pontemesh_home`; o armazenamento padrão será criado em
`/var/pontemesh_home/data/storage`. Para usar uma pasta específica do host,
monte essa pasta como volume em `/var/pontemesh_home`, ou defina
`PONTEMESH_STORAGE_PATH` para um caminho interno já preparado.

Também é possível executar com Docker:

```bash
docker compose -p ponte-mesh -f docker/docker-compose.yml up -d --build
```

O `docker-compose.yml` sobe o PostgreSQL e passa
`PONTEMESH_DATABASE_URL=postgres://pontemesh:pontemesh@postgres:5432/pontemesh`
para o servidor.

O Docker Compose sobe o projeto `ponte-mesh` com os serviços `server` e
`postgres`, agrupados como uma única aplicação no Docker Desktop. O PostgreSQL
fica na rede interna do Compose. Com PostgreSQL 18, o volume
`pontemesh_postgres` é montado em `/var/lib/postgresql`.

O Compose local usa `docker/Dockerfile.local`, que empacota o binário e o build
web já gerados pelo script. A imagem multi-stage em `docker/Dockerfile` constrói
frontend e backend dentro do próprio build da imagem.

Acesse:

```text
Painel web: http://localhost:8080
Endpoint S3-compatible: http://localhost:9000
```

## Comando único para construção e execução

Todos os comandos acima podem ser resumidos em um script único, que prepara o ambiente e abre o painel web:

```bash
./scripts/start-panel.sh
```

Esse comando executa o fluxo local:

* instala dependências e constrói o frontend;
* constrói o backend Rust em modo release;
* chama Docker Compose com o projeto `ponte-mesh`;
* constrói a imagem Docker pelo Compose;
* sobe `server` e `postgres` como serviços do mesmo projeto;
* aguarda o PostgreSQL saudável;
* espera o servidor responder;
* abre uma nova guia do navegador em `http://localhost:8080`.

O comando usa, por padrão:

```text
imagem Docker: pontemesh-server:local
projeto Compose: ponte-mesh
serviços: server, postgres
painel web: http://localhost:8080
endpoint S3-compatible: http://localhost:9000
```

É possível sobrescrever esses valores por variáveis de ambiente:

```bash
PONTEMESH_WEB_HOST_PORT=8081 \
./scripts/start-panel.sh
```

Nesse exemplo, o navegador será aberto em:

```text
http://localhost:8081
```

Para reiniciar um ambiente de desenvolvimento sem afetar outros projetos, use:

```bash
./scripts/start-panel.sh --reset-dev
```

Essa opção executa o reset do projeto Compose:

```bash
docker compose -p ponte-mesh -f docker/docker-compose.yml down --volumes --remove-orphans
```

Em produção, faça backup ou migração antes de remover volumes.

## Endpoint S3-compatible

O painel web/admin e a API S3-compatible usam portas separadas:

```text
Painel web/admin: http://localhost:8080
API S3-compatible: http://localhost:9000
```

Clientes S3 usam path-style com:

```text
endpoint_url = http://localhost:9000
```

As rotas S3-compatible ficam na raiz do endpoint S3. Clientes S3 devem trocar o
endpoint para `http://localhost:9000` e usar paths no formato `/{bucket}/{key}`.

SDKs Ponte Mesh podem usar pacotes de acesso temporários emitidos pelo Origin
para obter objetos por rotas próprias em `/pontemesh/access-packages/...`,
mantendo o endpoint S3-compatible separado para clientes S3.

Na primeira configuração, o painel gera uma access key S3 inicial para o admin e
mostra o segredo uma única vez. Depois disso, novas chaves podem ser criadas ou
revogadas em `Settings > S3 Access Keys`.

As variáveis de bootstrap são opcionais e servem apenas para importar uma chave
externa em cenários avançados:

```bash
export PONTEMESH_S3_BOOTSTRAP_ACCESS_KEY_ID=PMKEXTERNALACCESSKEY
export PONTEMESH_S3_BOOTSTRAP_SECRET_ACCESS_KEY='<secret-gerado-fora-do-pontemesh>'
./scripts/start-panel.sh
```

Exemplos com AWS CLI:

```bash
AWS_ACCESS_KEY_ID='<access-key-gerada-no-painel>' \
AWS_SECRET_ACCESS_KEY='<secret-exibido-uma-unica-vez>' \
aws --endpoint-url http://localhost:9000 s3api list-buckets
```

```bash
AWS_ACCESS_KEY_ID='<access-key-gerada-no-painel>' \
AWS_SECRET_ACCESS_KEY='<secret-exibido-uma-unica-vez>' \
aws --endpoint-url http://localhost:9000 s3api create-bucket --bucket test-bucket
```

```bash
AWS_ACCESS_KEY_ID='<access-key-gerada-no-painel>' \
AWS_SECRET_ACCESS_KEY='<secret-exibido-uma-unica-vez>' \
aws --endpoint-url http://localhost:9000 s3api put-object --bucket test-bucket --key hello.txt --body ./hello.txt
```

## Configuração inicial

Na primeira execução, o Ponte Mesh Server cria um token administrativo inicial.

O token é salvo em:

```text
/var/pontemesh_home/secrets/initialAdminToken
```

Ele também aparece nos logs do servidor.

Com Docker Compose, visualize os logs com:

```bash
docker compose -p ponte-mesh -f docker/docker-compose.yml logs server
```

Ou leia o token diretamente:

```bash
docker compose -p ponte-mesh -f docker/docker-compose.yml exec server cat /var/pontemesh_home/secrets/initialAdminToken
```

Depois, acesse:

```text
http://localhost:8080
```

Cole o token inicial no painel web e conclua a configuração da instância.

## Apoie o projeto

Se este projeto for útil para você, considere apoiar o desenvolvimento:

[Patrocinar no GitHub Sponsors](https://github.com/sponsors/fhfelipefh)
