use serde_json::{Value, json};

pub fn list_prompts() -> Value {
    json!({
        "prompts": [
            prompt("diagnose_instance", "Analisa o estado geral da instancia Ponte Mesh."),
            prompt("summarize_storage", "Resume o uso de storage e pontos de atencao."),
            prompt("analyze_bucket_growth", "Analisa crescimento e distribuicao de buckets."),
            prompt("review_recent_errors", "Revisa eventos recentes em busca de erros."),
            prompt("check_software_releases", "Instrui a verificacao de versoes e politicas de software em buckets versionados.")
        ]
    })
}

pub fn get_prompt(name: &str) -> anyhow::Result<Value> {
    let text = match name {
        "diagnose_instance" => {
            "Use os resources pontemesh://instance/status, pontemesh://instance/health e pontemesh://audit/recent para diagnosticar a instancia sem solicitar segredos."
        }
        "summarize_storage" => {
            "Use pontemesh://storage/summary e pontemesh://buckets para resumir uso de armazenamento e riscos operacionais."
        }
        "analyze_bucket_growth" => {
            "Use pontemesh://buckets e pontemesh://buckets/{bucketName}/objects para analisar crescimento por bucket com paginacao."
        }
        "review_recent_errors" => {
            "Use pontemesh://audit/recent para identificar falhas recentes, sem expor tokens ou credenciais."
        }
        "check_software_releases" => {
            "Use pontemesh_get_bucket_policy para verificar o esquema de versionamento e pontemesh_check_software_update para checar novas versoes de jogos/softwares."
        }
        _ => anyhow::bail!("unknown MCP prompt: {name}"),
    };
    Ok(json!({
        "description": text,
        "messages": [{
            "role": "user",
            "content": {
                "type": "text",
                "text": text
            }
        }]
    }))
}

fn prompt(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "description": description,
        "arguments": []
    })
}
