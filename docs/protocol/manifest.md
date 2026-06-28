# Manifesto

O **manifesto** é o documento emitido, assinado ou validado pelo Origin que descreve a estrutura de um objeto fragmentado.

Ele é a referência de autoridade para integridade, fragmentação, obtenção, fallback e remontagem lógica do objeto.

No Ponte Mesh, SDKs, peers e Replica/Edge não podem substituir, alterar ou redefinir as informações de integridade presentes no manifesto. O Origin permanece como autoridade sobre o objeto, sua versão, seus fragmentos, seus hashes e suas políticas de obtenção.

## Objetivo

O manifesto permite que o SDK obtenha um objeto de forma controlada e verificável.

Ele deve fornecer informações suficientes para que o SDK consiga:

* identificar o objeto;
* identificar a versão correta do objeto;
* conhecer o tamanho total esperado;
* conhecer a lista de fragmentos;
* mapear fragmentos para intervalos de bytes;
* validar cada fragmento por hash;
* preservar fragmentos já validados;
* aplicar fallback por fragmento ou intervalo;
* remontar logicamente o objeto;
* aplicar políticas de obtenção;
* respeitar expiração e revogação.

## Autoridade do manifesto

O manifesto deve ser emitido, assinado ou validado pelo Origin.

O manifesto é a fonte de verdade para:

* estrutura do objeto;
* lista de fragmentos;
* intervalos de bytes;
* tamanhos esperados;
* algoritmo de hash;
* hashes de fragmentos;
* hash do objeto completo, quando aplicável;
* versão do objeto;
* política de obtenção;
* validade;
* relação com o pacote de acesso.

Peers e Replica/Edge podem fornecer dados, mas não podem ser autoridade sobre a estrutura ou integridade do objeto.

## Campos conceituais

Um manifesto pode conter os seguintes campos conceituais:

* identificador do manifesto;
* `objectId`;
* bucket;
* chave do objeto;
* versão do objeto;
* tamanho total;
* tipo de conteúdo;
* metadados relevantes;
* algoritmo de hash;
* hash do objeto completo, quando aplicável;
* lista de fragmentos;
* política de obtenção;
* política de seleção de fontes;
* política de fallback;
* validade do manifesto;
* referência ao pacote de acesso;
* estado de disponibilidade do objeto;
* data de criação do manifesto;
* data de expiração do manifesto;
* assinatura ou proteção equivalente emitida pelo Origin;
* identificador de correlação para auditoria.

Esses campos são conceituais e podem evoluir durante a implementação, desde que preservem os requisitos de integridade, autorização, revogação e fallback.

## Identificação do objeto

O manifesto deve identificar claramente o objeto ao qual pertence.

Essa identificação pode ser feita por:

* `objectId`;
* bucket e chave;
* versão;
* identificador interno do catálogo;
* combinação desses campos.

O SDK não deve aplicar um manifesto a outro objeto, outro bucket, outra chave ou outra versão.

## Versão do objeto

O manifesto deve estar associado a uma versão específica do objeto.

Quando o objeto for alterado, substituído ou reprocessado, uma nova versão ou novo manifesto deve ser gerado.

Essa regra evita que fragmentos de versões diferentes sejam misturados durante a remontagem.

## Tamanho total

O manifesto deve informar o tamanho total esperado do objeto.

Esse valor permite ao SDK:

* validar a reconstrução final;
* calcular progresso;
* validar intervalos;
* detectar inconsistências;
* organizar fragmentos;
* aplicar recuperação parcial por `Range`.

## Tipo de conteúdo e metadados

O manifesto pode conter tipo de conteúdo e metadados relevantes.

Exemplos:

* `contentType`;
* nome original;
* tamanho;
* data de criação;
* data de modificação;
* versão;
* política aplicável;
* flags de uso progressivo;
* metadados necessários para consumo parcial.

Esses metadados não devem substituir as políticas do Origin, mas podem auxiliar o SDK e a aplicação consumidora.

## Algoritmo de hash

O manifesto deve declarar o algoritmo de hash utilizado para validar fragmentos e, quando aplicável, o objeto completo.

Exemplos conceituais:

* `SHA-256`;
* `SHA-512`;
* outro algoritmo seguro definido pela implementação.

A implementação deve usar bibliotecas criptográficas consolidadas da plataforma.

Não devem ser implementados algoritmos próprios de hash, comparação segura ou validação criptográfica.

## Hash do objeto completo

Quando aplicável, o manifesto pode conter o hash do objeto completo.

Esse hash permite validar a reconstrução final após todos os fragmentos terem sido obtidos e validados individualmente.

A validação do objeto completo não substitui a validação por fragmento. Ela pode ser uma camada adicional de verificação.

## Lista de fragmentos

O manifesto deve conter a lista de fragmentos que compõem o objeto.

Cada fragmento deve possuir informações suficientes para:

* ser identificado;
* ser solicitado;
* ser validado;
* ser associado à posição correta no objeto;
* ser reconstruído logicamente;
* ser recuperado por fallback;
* ser solicitado por intervalo de bytes ao Origin.

## Fragmento no manifesto

Cada fragmento deve conter, no mínimo:

* índice;
* identificador do fragmento;
* intervalo de bytes;
* tamanho esperado;
* hash esperado;
* prioridade ou classe de obtenção, quando aplicável.

Campos adicionais podem incluir:

* classe de criticidade;
* indicação de fragmento inicial;
* indicação de fragmento raro;
* dependências com outros fragmentos;
* possibilidade de retomada parcial;
* política específica de obtenção;
* fontes preferenciais, quando aplicável.

## Índice do fragmento

O índice define a posição lógica do fragmento dentro do objeto.

O SDK deve usar o índice para organizar a reconstrução final.

Fragmentos com índice duplicado, ausente ou fora do intervalo esperado devem ser rejeitados.

## Intervalo de bytes

O intervalo de bytes define a posição do fragmento dentro do objeto original.

Exemplo conceitual:

```text id="cwodgv"
byteRangeStart = 1048576
byteRangeEnd = 2097151
```

Esse intervalo permite recuperação parcial pelo Origin usando `Range`.

Também permite ao SDK reconstruir o objeto sem depender da ordem em que os fragmentos foram baixados.

## Tamanho esperado

Cada fragmento deve indicar seu tamanho esperado.

O SDK deve rejeitar fragmentos com tamanho diferente do esperado, salvo quando a política permitir armazenamento temporário parcial para retomada segura.

Dados parciais não devem ser tratados como fragmentos válidos.

## Hash esperado

Cada fragmento deve conter um hash esperado.

O SDK deve calcular o hash do conteúdo recebido e compará-lo com o hash informado no manifesto.

Se o hash não corresponder, o fragmento deve ser rejeitado.

Hashes fornecidos por peers ou Replica/Edge não devem substituir os hashes do manifesto.

## Prioridade ou classe de obtenção

O manifesto pode indicar prioridade ou classe de obtenção de cada fragmento.

Exemplos:

* fragmento inicial;
* cabeçalho;
* metadado crítico;
* fragmento de continuidade;
* fragmento raro;
* fragmento comum;
* fragmento final;
* fragmento problemático.

Essas informações podem auxiliar estratégias como:

* `headers-first`;
* `priority-first`;
* `rarest-first`;
* priorização sequencial;
* janela de continuidade;
* endgame.

## Política de obtenção

O manifesto pode referenciar ou conter políticas de obtenção aplicáveis.

Essas políticas podem definir:

* se P2P é permitido;
* se Replica/Edge é permitido;
* prioridade entre fontes;
* critérios de seleção de fragmentos;
* critérios de seleção de fontes;
* limites de timeout;
* limites de tentativas;
* regras de fallback;
* regras de revalidação;
* comportamento em caso de expiração;
* comportamento em caso de revogação.

A política pode estar diretamente no manifesto ou ser referenciada pelo pacote de acesso.

## Validade do manifesto

O manifesto pode possuir validade própria ou estar associado à validade do pacote de acesso.

Um manifesto expirado não deve autorizar novas transferências.

Um manifesto revogado não deve autorizar novas transferências.

Em transferências prolongadas, o SDK pode precisar revalidar o manifesto ou o pacote de acesso junto ao Origin.

## Relação com o pacote de acesso

O manifesto normalmente é usado em conjunto com o pacote de acesso.

O pacote de acesso define quem pode obter, por quanto tempo, com quais fontes e sob quais políticas.

O manifesto define o que deve ser obtido e como validar os fragmentos.

Em conjunto, eles permitem uma obtenção controlada, verificável e revogável.

## Proteção do manifesto

O manifesto deve ser protegido contra adulteração.

Mecanismos possíveis:

* assinatura digital;
* token assinado;
* hash protegido;
* referência segura validada pelo Origin;
* transporte autenticado;
* associação ao pacote de acesso;
* validação por bibliotecas criptográficas consolidadas.

A implementação não deve criar assinatura, criptografia ou proteção própria de forma manual.

## Regras de integridade

Regras obrigatórias:

* SDK, peers e Replica/Edge não podem substituir hashes do manifesto;
* fragmento só é concluído após validação por hash;
* fragmento inválido deve ser descartado;
* fragmento incompleto não deve ser marcado como concluído;
* dados parciais não são fragmentos válidos;
* o objeto final deve ser reconstruído apenas com fragmentos validados;
* fragmentos de versões diferentes não devem ser misturados;
* manifesto expirado não deve autorizar nova obtenção;
* manifesto revogado não deve autorizar nova obtenção;
* alteração de objeto deve gerar nova versão ou novo manifesto.

## Revogação

O Origin deve poder revogar um manifesto direta ou indiretamente.

A revogação pode ocorrer por:

* revogação do objeto;
* revogação do pacote de acesso;
* revogação do usuário;
* revogação da aplicação;
* revogação de uma política;
* alteração de versão do objeto;
* remoção lógica do objeto;
* detecção de inconsistência;
* decisão administrativa.

Após revogação, o manifesto não deve ser usado para novas transferências.

## Expiração

O manifesto pode expirar conforme política do Origin.

Após a expiração:

* novas obtenções não devem ser iniciadas com esse manifesto;
* o SDK deve consultar o Origin novamente, se a política permitir;
* Replica/Edge não deve usar o manifesto expirado como base para servir conteúdo;
* peers não devem ser considerados autorizados com base em manifesto expirado.

## Alteração de objeto

Quando um objeto for alterado, o Origin deve gerar uma nova versão ou um novo manifesto.

O SDK não deve misturar fragmentos de manifestos diferentes.

Replica/Edge deve sincronizar novamente o conteúdo quando a política indicar que a versão anterior não é mais válida.

## Relação com o SDK

O SDK deve usar o manifesto para:

* criar o mapa local de fragmentos;
* controlar progresso;
* selecionar fragmentos;
* selecionar fontes conforme política;
* validar hashes;
* detectar fragmentos inválidos;
* aplicar fallback;
* preservar fragmentos validados;
* reconstruir o objeto final.

O SDK não deve aceitar manifesto de peer ou Replica/Edge como autoridade.

## Relação com Replica/Edge

Replica/Edge pode usar o manifesto para validar conteúdo sincronizado e anunciar disponibilidade.

No entanto:

* não pode alterar o manifesto;
* não pode emitir manifesto como autoridade;
* não pode substituir hashes;
* não pode servir fragmentos fora do escopo autorizado;
* deve respeitar revogação e expiração do manifesto ou pacote associado.

## Relação com peers

Peers podem fornecer fragmentos temporariamente quando autorizados pela política.

No entanto:

* peer não define manifesto;
* peer não define hash;
* peer não define versão;
* peer não define política;
* peer não deve ser autoridade de integridade;
* fragmentos vindos de peer devem ser validados conforme manifesto do Origin.

## Manifesto e fallback

O manifesto é essencial para fallback granular.

Como ele define intervalos de bytes, tamanhos e hashes, o SDK pode identificar exatamente quais fragmentos estão ausentes, inválidos ou problemáticos.

Com isso, o SDK pode solicitar ao Origin apenas os fragmentos ou intervalos necessários, preservando fragmentos já validados.

## Manifesto e recuperação por Range

O manifesto deve ser compatível com recuperação por intervalo de bytes.

Cada fragmento deve mapear claramente:

* início do intervalo;
* fim do intervalo;
* tamanho esperado;
* hash esperado.

Isso permite que o Origin atenda requisições `Range` para fallback, retomada parcial e obtenção de fragmentos específicos.

## Auditoria

Eventos relacionados ao manifesto podem ser auditados.

Eventos recomendados:

* manifesto gerado;
* manifesto consultado;
* manifesto assinado;
* manifesto expirado;
* manifesto revogado;
* manifesto rejeitado;
* tentativa de uso de manifesto inválido;
* tentativa de uso de manifesto expirado;
* tentativa de uso de manifesto de versão incompatível;
* divergência entre fragmento e manifesto;
* alteração de objeto que gerou novo manifesto.

A auditoria não deve expor segredos, tokens completos ou conteúdo do objeto.

## Segurança

O manifesto é um artefato sensível.

Regras de segurança:

* deve ser emitido, assinado ou validado pelo Origin;
* deve ser protegido contra adulteração;
* deve possuir relação clara com objeto, versão e pacote de acesso;
* deve expirar ou estar associado a uma autorização temporária;
* deve ser revogável;
* não deve ser aceito se for inválido, expirado ou incompatível;
* não deve conter segredos permanentes;
* não deve permitir acesso fora do escopo autorizado;
* deve usar proteção criptográfica implementada por bibliotecas consolidadas.

## Exemplo conceitual

Exemplo simplificado de manifesto:

```json id="8y5xua"
{
  "manifestId": "manifest_123",
  "objectId": "object_456",
  "bucket": "videos",
  "key": "aula-01.mp4",
  "version": "v3",
  "size": 104857600,
  "contentType": "video/mp4",
  "hashAlgorithm": "SHA-256",
  "objectHash": "hash-do-objeto-completo",
  "validUntil": "2026-12-31T23:59:59Z",
  "fragments": [
    {
      "index": 0,
      "fragmentId": "fragment_0",
      "byteRangeStart": 0,
      "byteRangeEnd": 1048575,
      "expectedSize": 1048576,
      "expectedHash": "hash-do-fragmento-0",
      "priorityClass": "INITIAL"
    },
    {
      "index": 1,
      "fragmentId": "fragment_1",
      "byteRangeStart": 1048576,
      "byteRangeEnd": 2097151,
      "expectedSize": 1048576,
      "expectedHash": "hash-do-fragmento-1",
      "priorityClass": "CONTINUITY"
    }
  ],
  "policyRef": "policy_789",
  "signature": "assinatura-gerada-pelo-origin"
}
```

Esse exemplo é apenas conceitual. O formato final deve ser definido durante a implementação.

## Síntese

O manifesto é o documento que descreve como um objeto fragmentado deve ser obtido, validado e remontado.

Ele deve ser emitido, assinado ou validado pelo Origin.

Ele é a autoridade sobre fragmentos, intervalos, tamanhos, hashes, versão e políticas de obtenção.

SDKs, peers e Replica/Edge não podem substituir hashes do manifesto.

Fragmentos só podem ser aceitos após validação de integridade.

Manifestos expirados, revogados ou incompatíveis não devem autorizar novas transferências.
