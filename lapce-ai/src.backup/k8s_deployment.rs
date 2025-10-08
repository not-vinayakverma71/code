/// Kubernetes Deployment Manifests - Day 39 AM
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
pub struct K8sDeployment {
    #[serde(rename = "apiVersion")]
    api_version: String,
    kind: String,
    metadata: Metadata,
    spec: DeploymentSpec,
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    name: String,
    namespace: String,
    labels: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct DeploymentSpec {
    replicas: i32,
    selector: Selector,
    template: PodTemplate,
}

#[derive(Serialize, Deserialize)]
pub struct Selector {
    #[serde(rename = "matchLabels")]
    match_labels: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct PodTemplate {
    metadata: Metadata,
    spec: PodSpec,
}

#[derive(Serialize, Deserialize)]
pub struct PodSpec {
    containers: Vec<Container>,
    volumes: Vec<Volume>,
}

#[derive(Serialize, Deserialize)]
pub struct Container {
    name: String,
    image: String,
    ports: Vec<ContainerPort>,
    env: Vec<EnvVar>,
    resources: Resources,
    #[serde(rename = "volumeMounts")]
    volume_mounts: Vec<VolumeMount>,
    #[serde(rename = "livenessProbe")]
    liveness_probe: Option<Probe>,
    #[serde(rename = "readinessProbe")]
    readiness_probe: Option<Probe>,
}

#[derive(Serialize, Deserialize)]
pub struct ContainerPort {
    name: String,
    #[serde(rename = "containerPort")]
    container_port: i32,
    protocol: String,
}

#[derive(Serialize, Deserialize)]
pub struct EnvVar {
    name: String,
    value: Option<String>,
    #[serde(rename = "valueFrom")]
    value_from: Option<ValueFrom>,
}

#[derive(Serialize, Deserialize)]
pub struct ValueFrom {
    #[serde(rename = "secretKeyRef")]
    secret_key_ref: Option<SecretKeyRef>,
}

#[derive(Serialize, Deserialize)]
pub struct SecretKeyRef {
    name: String,
    key: String,
}

#[derive(Serialize, Deserialize)]
pub struct Resources {
    limits: BTreeMap<String, String>,
    requests: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct Volume {
    name: String,
    #[serde(rename = "persistentVolumeClaim")]
    persistent_volume_claim: Option<PersistentVolumeClaim>,
    #[serde(rename = "configMap")]
    config_map: Option<ConfigMapVolume>,
}

#[derive(Serialize, Deserialize)]
pub struct PersistentVolumeClaim {
    #[serde(rename = "claimName")]
    claim_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct ConfigMapVolume {
    name: String,
}

#[derive(Serialize, Deserialize)]
pub struct VolumeMount {
    name: String,
    #[serde(rename = "mountPath")]
    mount_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct Probe {
    #[serde(rename = "httpGet")]
    http_get: Option<HttpGetAction>,
    #[serde(rename = "initialDelaySeconds")]
    initial_delay_seconds: i32,
    #[serde(rename = "periodSeconds")]
    period_seconds: i32,
}

#[derive(Serialize, Deserialize)]
pub struct HttpGetAction {
    path: String,
    port: i32,
}

pub fn create_deployment() -> K8sDeployment {
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), "lapce-ai-rust".to_string());
    
    let mut resources = BTreeMap::new();
    resources.insert("memory".to_string(), "512Mi".to_string());
    resources.insert("cpu".to_string(), "500m".to_string());
    
    let mut limits = resources.clone();
    limits.insert("memory".to_string(), "1Gi".to_string());
    limits.insert("cpu".to_string(), "1000m".to_string());
    
    K8sDeployment {
        api_version: "apps/v1".to_string(),
        kind: "Deployment".to_string(),
        metadata: Metadata {
            name: "lapce-ai-rust".to_string(),
            namespace: "default".to_string(),
            labels: labels.clone(),
        },
        spec: DeploymentSpec {
            replicas: 3,
            selector: Selector {
                match_labels: labels.clone(),
            },
            template: PodTemplate {
                metadata: Metadata {
                    name: "lapce-ai-rust".to_string(),
                    namespace: "default".to_string(),
                    labels,
                },
                spec: PodSpec {
                    containers: vec![Container {
                        name: "lapce-ai-rust".to_string(),
                        image: "lapce/ai-rust:latest".to_string(),
                        ports: vec![
                            ContainerPort {
                                name: "grpc".to_string(),
                                container_port: 50051,
                                protocol: "TCP".to_string(),
                            },
                            ContainerPort {
                                name: "http".to_string(),
                                container_port: 8080,
                                protocol: "TCP".to_string(),
                            },
                        ],
                        env: vec![
                            EnvVar {
                                name: "RUST_LOG".to_string(),
                                value: Some("info".to_string()),
                                value_from: None,
                            },
                        ],
                        resources: Resources {
                            requests: resources,
                            limits,
                        },
                        volume_mounts: vec![
                            VolumeMount {
                                name: "data".to_string(),
                                mount_path: "/data".to_string(),
                            },
                        ],
                        liveness_probe: Some(Probe {
                            http_get: Some(HttpGetAction {
                                path: "/health".to_string(),
                                port: 8080,
                            }),
                            initial_delay_seconds: 30,
                            period_seconds: 10,
                        }),
                        readiness_probe: Some(Probe {
                            http_get: Some(HttpGetAction {
                                path: "/ready".to_string(),
                                port: 8080,
                            }),
                            initial_delay_seconds: 5,
                            period_seconds: 5,
                        }),
                    }],
                    volumes: vec![
                        Volume {
                            name: "data".to_string(),
                            persistent_volume_claim: Some(PersistentVolumeClaim {
                                claim_name: "lapce-ai-rust-pvc".to_string(),
                            }),
                            config_map: None,
                        },
                    ],
                },
            },
        },
    }
}

pub fn generate_yaml() -> String {
    let deployment = create_deployment();
    serde_yaml::to_string(&deployment).unwrap()
}
