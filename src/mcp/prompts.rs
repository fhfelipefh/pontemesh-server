use serde_json::{Value, json};

pub fn list_prompts() -> Value {
    json!({
        "prompts": [
            prompt("diagnose_instance", "Analisa o estado geral da instancia Ponte Mesh."),
            prompt("summarize_storage", "Resume o uso de storage e pontos de atencao."),
            prompt("analyze_bucket_growth", "Analisa crescimento e distribuicao de buckets."),
            prompt("review_recent_errors", "Revisa eventos recentes em busca de erros."),
            prompt("check_software_releases", "Instrui a verificacao de versoes e politicas de software em buckets versionados."),
            prompt("analyze_egress_offload", "Analisa as metricas de economia de banda e offload de egress."),
            prompt("manage_storage_drives", "Orienta a expansao do pool de discos e substituicao a quente com drenagem de objetos.")
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
            "Use pontemesh_get_bucket_policy para verificar o esquema de versionamento, pontemesh_check_software_update para checar novas versoes de jogos/softwares e pontemesh_get_version_check_metrics ou pontemesh://metrics/version-checks para auditar o volume de consultas recebidas."
        }
        "analyze_egress_offload" => {
            "Use pontemesh://metrics/offload e pontemesh_get_offload_metrics para analisar a porcentagem de dados transferidos por Peers e Replicas, calculando a economia de banda em relacao ao Origin."
        }
        "manage_storage_drives" => {
            "Use pontemesh://storage/drives e pontemesh_list_storage_drives para inspecionar os discos do pool, pontemesh_add_storage_drive para adicionar novos discos sem reiniciar o servidor e pontemesh_drain_storage_drive para migrar dados de um disco antes da remocao."
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
