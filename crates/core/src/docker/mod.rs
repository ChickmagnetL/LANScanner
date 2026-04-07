use std::fmt;
use std::time::Duration;

use crate::ssh::auth;

const DOCKER_TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub is_running: bool,
}

#[derive(Debug)]
pub enum DockerError {
    Ssh(auth::SshError),
    Parse(String),
    Runtime(String),
}

impl fmt::Display for DockerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ssh(error) => write!(f, "{error}"),
            Self::Parse(message) => write!(f, "{message}"),
            Self::Runtime(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for DockerError {}

impl From<auth::SshError> for DockerError {
    fn from(error: auth::SshError) -> Self {
        Self::Ssh(error)
    }
}

pub async fn list_containers(
    ip: &str,
    user: &str,
    password: Option<&str>,
) -> Result<Vec<Container>, DockerError> {
    let output = auth::execute_remote_command(
        ip,
        user,
        password,
        DOCKER_TIMEOUT,
        "docker ps -a --no-trunc --format '{{.ID}}|{{.Names}}|{{.Image}}|{{.Status}}'",
    )
    .await?;

    if output.exit_status != 0 {
        return Err(DockerError::Runtime(map_docker_error(
            &output.stdout,
            &output.stderr,
            "未能读取远程 Docker 容器列表",
        )));
    }

    let mut containers = Vec::new();
    for line in output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let parts = line.split('|').map(str::trim).collect::<Vec<_>>();
        if parts.len() != 4 {
            return Err(DockerError::Parse(format!(
                "unexpected docker ps output line: {line}"
            )));
        }

        let status = parts[3].to_owned();
        let normalized = status.to_ascii_lowercase();
        containers.push(Container {
            id: parts[0].to_owned(),
            name: parts[1].to_owned(),
            image: parts[2].to_owned(),
            is_running: normalized.starts_with("up"),
            status,
        });
    }

    if containers.is_empty() {
        return Err(DockerError::Runtime(String::from(
            "未在远程主机上发现 Docker 容器",
        )));
    }

    Ok(containers)
}

pub async fn restart_container(
    ip: &str,
    user: &str,
    password: Option<&str>,
    container_id: &str,
) -> Result<(), DockerError> {
    let command = format!("docker start {}", shell_quote(container_id));
    let output = auth::execute_remote_command(ip, user, password, DOCKER_TIMEOUT, &command).await?;

    if output.exit_status == 0 {
        Ok(())
    } else {
        Err(DockerError::Runtime(map_docker_error(
            &output.stdout,
            &output.stderr,
            "启动 Docker 容器失败",
        )))
    }
}

pub fn build_devcontainer_uri(host_target: &str, user: &str, container_name: &str) -> String {
    build_devcontainer_uri_with_workdir(host_target, user, container_name, "/")
}

pub async fn prepare_devcontainer_uri(
    host_target: &str,
    ip: &str,
    user: &str,
    password: Option<&str>,
    container_id: &str,
    container_name: &str,
) -> Result<String, DockerError> {
    let container_user = inspect_container_user(ip, user, password, container_id)
        .await
        .unwrap_or_else(|_| String::from("root"));
    let workdir = inspect_container_workdir(ip, user, password, container_id, &container_user)
        .await
        .unwrap_or_else(|_| String::from("/"));

    Ok(build_devcontainer_uri_with_workdir(
        host_target,
        user,
        container_name,
        &workdir,
    ))
}

fn build_devcontainer_uri_with_workdir(
    host_target: &str,
    user: &str,
    container_name: &str,
    workdir: &str,
) -> String {
    let payload = format!(
        "{{\"containerName\":\"/{}\",\"settings\":{{\"host\":\"ssh://{}@{}:22\"}}}}",
        json_escape(container_name),
        json_escape(user),
        json_escape(host_target),
    );

    format!(
        "vscode-remote://attached-container+{}{}",
        hex_encode(payload.as_bytes()),
        workdir,
    )
}

async fn inspect_container_user(
    ip: &str,
    user: &str,
    password: Option<&str>,
    container_id: &str,
) -> Result<String, DockerError> {
    let command = format!(
        "docker inspect --format '{{{{.Config.User}}}}' {}",
        shell_quote(container_id)
    );
    let output = auth::execute_remote_command(ip, user, password, DOCKER_TIMEOUT, &command).await?;

    if output.exit_status != 0 {
        return Err(DockerError::Runtime(map_docker_error(
            &output.stdout,
            &output.stderr,
            "读取容器默认用户失败",
        )));
    }

    let value = output.stdout.trim();
    if value.is_empty() || value.chars().all(|value| value.is_ascii_digit()) {
        return Ok(String::from("root"));
    }

    Ok(value.split(':').next().unwrap_or("root").to_owned())
}

async fn inspect_container_workdir(
    ip: &str,
    user: &str,
    password: Option<&str>,
    container_id: &str,
    container_user: &str,
) -> Result<String, DockerError> {
    let expected_dir = if container_user == "root" {
        String::from("/root")
    } else {
        format!("/home/{container_user}")
    };
    let command = format!(
        "docker exec {} test -d {}",
        shell_quote(container_id),
        shell_quote(&expected_dir)
    );
    let output = auth::execute_remote_command(ip, user, password, DOCKER_TIMEOUT, &command).await?;

    if output.exit_status == 0 {
        Ok(expected_dir)
    } else {
        Ok(String::from("/"))
    }
}

fn map_docker_error(stdout: &str, stderr: &str, fallback: &str) -> String {
    let stderr = stderr.trim();
    let stderr_lower = stderr.to_ascii_lowercase();

    if stderr_lower.contains("command not found") || stderr_lower.contains("not recognized") {
        return String::from("远程主机未安装 Docker");
    }
    if stderr_lower.contains("permission denied") {
        return String::from("无权访问 Docker，请检查当前用户是否在 docker 组");
    }
    if stdout.trim().is_empty() && stderr.is_empty() {
        return String::from("未在远程主机上发现 Docker 容器");
    }
    if !stderr.is_empty() {
        return stderr.to_owned();
    }

    fallback.to_owned()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::build_devcontainer_uri;

    #[test]
    fn builds_expected_attached_container_uri() {
        let uri = build_devcontainer_uri("192.168.1.8", "root", "demo");

        assert!(uri.starts_with("vscode-remote://attached-container+"));
        assert!(uri.ends_with('/'));
    }
}
