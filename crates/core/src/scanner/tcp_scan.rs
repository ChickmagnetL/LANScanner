use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use ipnet::Ipv4Net;
use tokio::net::TcpSocket;
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

pub const SSH_PORT: u16 = 22;
pub const SCAN_CONCURRENCY: usize = 88;
pub const SCAN_TIMEOUT: Duration = Duration::from_millis(880);
const SCAN_CONCURRENCY_BURST: usize = 112;
const SCAN_TIMEOUT_FAST: Duration = Duration::from_millis(760);
const SCAN_HOST_BURST_THRESHOLD: usize = 160;
const SCAN_HOST_FAST_TIMEOUT_THRESHOLD: usize = 192;
const RETRY_SCAN_CONCURRENCY: usize = 32;
const RETRY_SCAN_TIMEOUT: Duration = Duration::from_millis(1200);
const RETRY_SCAN_CONCURRENCY_BURST: usize = 48;
const RETRY_SCAN_TIMEOUT_FAST: Duration = Duration::from_millis(1000);
const RETRY_HOST_BURST_THRESHOLD: usize = 48;
const RETRY_HOST_FAST_TIMEOUT_THRESHOLD: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TcpProbeReport {
    pub candidate_hosts: Vec<String>,
    pub open_hosts: Vec<String>,
    pub closed_hosts: Vec<String>,
    pub retry_exhausted_hosts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeOutcome {
    Open,
    RetryableTimeout,
    Closed,
}

pub async fn scan_subnet<F>(
    local_ip: &str,
    subnet: &str,
    candidate_ips: Vec<String>,
    on_progress: F,
    cancel_token: CancellationToken,
) -> Vec<String>
where
    F: Fn(usize, usize) + Send + Sync + 'static,
{
    scan_subnet_report(local_ip, subnet, candidate_ips, on_progress, cancel_token)
        .await
        .open_hosts
}

pub async fn scan_subnet_report<F>(
    local_ip: &str,
    subnet: &str,
    candidate_ips: Vec<String>,
    on_progress: F,
    cancel_token: CancellationToken,
) -> TcpProbeReport
where
    F: Fn(usize, usize) + Send + Sync + 'static,
{
    let local_ip_addr = match local_ip.parse::<Ipv4Addr>() {
        Ok(local_ip) => local_ip,
        Err(_) => return TcpProbeReport::default(),
    };

    let subnet_cidr = match subnet.parse::<Ipv4Net>() {
        Ok(subnet) => subnet,
        Err(_) => return TcpProbeReport::default(),
    };

    let candidate_hosts = normalize_probe_targets(candidate_ips, local_ip_addr, subnet_cidr);
    let total = candidate_hosts.len();

    if total == 0 {
        on_progress(0, 0);
        return TcpProbeReport::default();
    }

    let on_progress = Arc::new(on_progress);
    let scanned = Arc::new(AtomicUsize::new(0));
    let unbound_only = Arc::new(AtomicBool::new(false));
    let first_pass_targets = Arc::new(candidate_hosts.clone());

    let Some(mut first_pass_result) = run_probe_round(
        local_ip_addr,
        Arc::clone(&first_pass_targets),
        scan_worker_count(
            first_pass_targets.len(),
            scan_concurrency_limit(first_pass_targets.len()),
        ),
        scan_timeout(first_pass_targets.len()),
        false,
        total,
        Arc::clone(&scanned),
        Arc::clone(&on_progress),
        cancel_token.clone(),
        Arc::clone(&unbound_only),
    )
    .await
    else {
        return TcpProbeReport::default();
    };

    let mut open_hosts = first_pass_result.open_hosts;
    let mut retry_targets = Vec::new();

    if !first_pass_result.retry_hosts.is_empty() {
        retry_targets = std::mem::take(&mut first_pass_result.retry_hosts);
        let retry_count = retry_targets.len();
        let retry_round_targets = Arc::new(retry_targets.clone());
        let Some(mut retry_result) = run_probe_round(
            local_ip_addr,
            Arc::clone(&retry_round_targets),
            retry_worker_count(retry_round_targets.len()),
            retry_scan_timeout(retry_count),
            true,
            total,
            Arc::clone(&scanned),
            Arc::clone(&on_progress),
            cancel_token,
            unbound_only,
        )
        .await
        else {
            return TcpProbeReport::default();
        };

        open_hosts.append(&mut retry_result.open_hosts);
    }

    build_probe_report(candidate_hosts, open_hosts, retry_targets)
}

fn normalize_probe_targets(
    candidate_ips: Vec<String>,
    local_ip: Ipv4Addr,
    subnet: Ipv4Net,
) -> Vec<Ipv4Addr> {
    let mut filtered = candidate_ips
        .into_iter()
        .filter_map(|candidate| candidate.parse::<Ipv4Addr>().ok())
        .filter(|ip| {
            *ip != local_ip
                && is_valid_probe_candidate_ip(*ip)
                && is_subnet_host_address(subnet, *ip)
        })
        .collect::<Vec<_>>();
    sort_and_dedup_hosts(&mut filtered);
    filtered
}

fn is_subnet_host_address(subnet: Ipv4Net, ip: Ipv4Addr) -> bool {
    if !subnet.contains(&ip) {
        return false;
    }

    if subnet.prefix_len() < 31 {
        let network = subnet.network();
        let broadcast = subnet.broadcast();
        if ip == network || ip == broadcast {
            return false;
        }
    }

    true
}

fn is_valid_probe_candidate_ip(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified()
}

async fn probe_host(
    local_ip: Ipv4Addr,
    target_ip: Ipv4Addr,
    cancel_token: CancellationToken,
    scan_timeout: Duration,
    unbound_only: &AtomicBool,
) -> ProbeOutcome {
    if cancel_token.is_cancelled() {
        return ProbeOutcome::Closed;
    }

    tokio::select! {
        _ = cancel_token.cancelled() => ProbeOutcome::Closed,
        result = connect_once(local_ip, target_ip, scan_timeout, unbound_only) => result,
    }
}

async fn connect_once(
    local_ip: Ipv4Addr,
    target_ip: Ipv4Addr,
    scan_timeout: Duration,
    unbound_only: &AtomicBool,
) -> ProbeOutcome {
    let target = SocketAddr::new(IpAddr::V4(target_ip), SSH_PORT);

    if unbound_only.load(Ordering::Relaxed) {
        return connect_unbound(target, scan_timeout).await;
    }

    match create_bound_socket(local_ip) {
        Ok(socket) => connect_bound(socket, target, scan_timeout).await,
        Err(error) if should_retry_without_bind(&error) => {
            unbound_only.store(true, Ordering::Relaxed);
            connect_unbound(target, scan_timeout).await
        }
        Err(_) => ProbeOutcome::Closed,
    }
}

async fn run_probe_round<F>(
    local_ip: Ipv4Addr,
    targets: Arc<Vec<Ipv4Addr>>,
    worker_count: usize,
    scan_timeout: Duration,
    count_timeout_in_progress: bool,
    progress_total: usize,
    scanned: Arc<AtomicUsize>,
    on_progress: Arc<F>,
    cancel_token: CancellationToken,
    unbound_only: Arc<AtomicBool>,
) -> Option<ProbeRoundResult>
where
    F: Fn(usize, usize) + Send + Sync + 'static,
{
    if targets.is_empty() || worker_count == 0 {
        return Some(ProbeRoundResult::default());
    }

    let target_total = targets.len();
    let mut tasks = JoinSet::new();
    let next_host_index = Arc::new(AtomicUsize::new(0));

    for _ in 0..worker_count {
        if cancel_token.is_cancelled() {
            abort_and_drain(&mut tasks).await;
            return None;
        }

        let targets = Arc::clone(&targets);
        let next_host_index = Arc::clone(&next_host_index);
        let scanned = Arc::clone(&scanned);
        let on_progress = Arc::clone(&on_progress);
        let cancel_token = cancel_token.clone();
        let unbound_only = Arc::clone(&unbound_only);

        tasks.spawn(async move {
            let mut worker_results = Vec::new();

            loop {
                if cancel_token.is_cancelled() {
                    break;
                }

                let host_idx = next_host_index.fetch_add(1, Ordering::Relaxed);
                if host_idx >= target_total {
                    break;
                }

                let host = targets[host_idx];
                let outcome = probe_host(
                    local_ip,
                    host,
                    cancel_token.clone(),
                    scan_timeout,
                    &unbound_only,
                )
                .await;

                if cancel_token.is_cancelled() {
                    break;
                }

                if should_report_progress(outcome, count_timeout_in_progress) {
                    let scanned_now = scanned.fetch_add(1, Ordering::Relaxed) + 1;
                    on_progress(scanned_now, progress_total);
                }

                worker_results.push((host, outcome));
            }

            worker_results
        });
    }

    let mut round_results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        if cancel_token.is_cancelled() {
            abort_and_drain(&mut tasks).await;
            return None;
        }

        if let Ok(mut worker_results) = result {
            round_results.append(&mut worker_results);
        }
    }

    let open_hosts = collect_open_hosts(&round_results);
    let retry_hosts = collect_retry_targets(&round_results);
    Some(ProbeRoundResult {
        open_hosts,
        retry_hosts,
    })
}

fn scan_worker_count(total_hosts: usize, limit: usize) -> usize {
    total_hosts.min(limit)
}

fn scan_concurrency_limit(total_hosts: usize) -> usize {
    if total_hosts >= SCAN_HOST_BURST_THRESHOLD {
        SCAN_CONCURRENCY_BURST
    } else {
        SCAN_CONCURRENCY
    }
}

fn scan_timeout(total_hosts: usize) -> Duration {
    if total_hosts >= SCAN_HOST_FAST_TIMEOUT_THRESHOLD {
        SCAN_TIMEOUT_FAST
    } else {
        SCAN_TIMEOUT
    }
}

fn retry_worker_count(retry_hosts: usize) -> usize {
    let retry_limit = if retry_hosts >= RETRY_HOST_BURST_THRESHOLD {
        RETRY_SCAN_CONCURRENCY_BURST
    } else {
        RETRY_SCAN_CONCURRENCY
    };
    scan_worker_count(retry_hosts, retry_limit)
}

fn retry_scan_timeout(retry_hosts: usize) -> Duration {
    if retry_hosts >= RETRY_HOST_FAST_TIMEOUT_THRESHOLD {
        RETRY_SCAN_TIMEOUT_FAST
    } else {
        RETRY_SCAN_TIMEOUT
    }
}

async fn abort_and_drain<T: 'static>(tasks: &mut JoinSet<T>) {
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
}

fn create_bound_socket(local_ip: Ipv4Addr) -> std::io::Result<TcpSocket> {
    let socket = TcpSocket::new_v4()?;
    socket.bind(SocketAddr::new(IpAddr::V4(local_ip), 0))?;
    Ok(socket)
}

async fn connect_bound(
    socket: TcpSocket,
    target: SocketAddr,
    scan_timeout: Duration,
) -> ProbeOutcome {
    connect_with_timeout(socket.connect(target), scan_timeout).await
}

async fn connect_unbound(target: SocketAddr, scan_timeout: Duration) -> ProbeOutcome {
    let unbound_result = async {
        let socket = TcpSocket::new_v4()?;
        socket.connect(target).await
    };

    connect_with_timeout(unbound_result, scan_timeout).await
}

async fn connect_with_timeout<F>(future: F, scan_timeout: Duration) -> ProbeOutcome
where
    F: std::future::Future<Output = std::io::Result<tokio::net::TcpStream>>,
{
    match timeout(scan_timeout, future).await {
        Ok(Ok(_)) => ProbeOutcome::Open,
        Ok(Err(error)) => classify_connect_error(&error),
        Err(_) => ProbeOutcome::RetryableTimeout,
    }
}

fn classify_connect_error(error: &std::io::Error) -> ProbeOutcome {
    match error.kind() {
        std::io::ErrorKind::TimedOut => ProbeOutcome::RetryableTimeout,
        std::io::ErrorKind::ConnectionRefused
        | std::io::ErrorKind::NetworkUnreachable
        | std::io::ErrorKind::HostUnreachable => ProbeOutcome::Closed,
        _ => ProbeOutcome::Closed,
    }
}

fn should_retry_without_bind(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::AddrNotAvailable
            | std::io::ErrorKind::AddrInUse
            | std::io::ErrorKind::InvalidInput
            | std::io::ErrorKind::PermissionDenied
    )
}

fn should_report_progress(outcome: ProbeOutcome, count_timeout_in_progress: bool) -> bool {
    !matches!(outcome, ProbeOutcome::RetryableTimeout) || count_timeout_in_progress
}

fn collect_open_hosts(results: &[(Ipv4Addr, ProbeOutcome)]) -> Vec<Ipv4Addr> {
    results
        .iter()
        .filter_map(|(ip, outcome)| {
            if matches!(outcome, ProbeOutcome::Open) {
                Some(*ip)
            } else {
                None
            }
        })
        .collect()
}

fn collect_retry_targets(results: &[(Ipv4Addr, ProbeOutcome)]) -> Vec<Ipv4Addr> {
    results
        .iter()
        .filter_map(|(ip, outcome)| {
            if matches!(outcome, ProbeOutcome::RetryableTimeout) {
                Some(*ip)
            } else {
                None
            }
        })
        .collect()
}

fn build_probe_report(
    mut candidate_hosts: Vec<Ipv4Addr>,
    mut open_hosts: Vec<Ipv4Addr>,
    mut retry_hosts: Vec<Ipv4Addr>,
) -> TcpProbeReport {
    sort_and_dedup_hosts(&mut candidate_hosts);
    sort_and_dedup_hosts(&mut open_hosts);
    sort_and_dedup_hosts(&mut retry_hosts);

    let open_set = open_hosts.iter().copied().collect::<HashSet<_>>();

    let closed_hosts = candidate_hosts
        .iter()
        .filter(|ip| !open_set.contains(ip))
        .copied()
        .collect::<Vec<_>>();
    let retry_exhausted_hosts = retry_hosts
        .into_iter()
        .filter(|ip| !open_set.contains(ip))
        .collect::<Vec<_>>();

    TcpProbeReport {
        candidate_hosts: hosts_to_strings(candidate_hosts),
        open_hosts: hosts_to_strings(open_hosts),
        closed_hosts: hosts_to_strings(closed_hosts),
        retry_exhausted_hosts: hosts_to_strings(retry_exhausted_hosts),
    }
}

fn hosts_to_strings(hosts: Vec<Ipv4Addr>) -> Vec<String> {
    hosts.into_iter().map(|ip| ip.to_string()).collect()
}

fn sort_and_dedup_hosts(hosts: &mut Vec<Ipv4Addr>) {
    hosts.sort_unstable();
    hosts.dedup();
}

#[derive(Default)]
struct ProbeRoundResult {
    open_hosts: Vec<Ipv4Addr>,
    retry_hosts: Vec<Ipv4Addr>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_count_is_capped_by_concurrency() {
        assert_eq!(scan_worker_count(0, SCAN_CONCURRENCY), 0);
        assert_eq!(scan_worker_count(1, SCAN_CONCURRENCY), 1);
        assert_eq!(
            scan_worker_count(SCAN_CONCURRENCY, SCAN_CONCURRENCY),
            SCAN_CONCURRENCY
        );
        assert_eq!(
            scan_worker_count(SCAN_CONCURRENCY + 10, SCAN_CONCURRENCY),
            SCAN_CONCURRENCY
        );
    }

    #[test]
    fn probe_targets_only_use_caller_provided_candidates() {
        let subnet = "192.168.31.0/24".parse::<Ipv4Net>().unwrap();
        let local_ip = "192.168.31.8".parse::<Ipv4Addr>().unwrap();
        let targets = normalize_probe_targets(
            vec![
                String::from("10.0.0.5"),
                String::from("192.168.31.8"),
                String::from("192.168.31.40"),
                String::from("192.168.31.12"),
                String::from("192.168.31.12"),
                String::from("169.254.2.9"),
                String::from("not-an-ip"),
            ],
            local_ip,
            subnet,
        );
        assert_eq!(
            targets,
            vec![
                "192.168.31.12".parse::<Ipv4Addr>().unwrap(),
                "192.168.31.40".parse::<Ipv4Addr>().unwrap(),
            ]
        );
    }

    #[test]
    fn empty_candidate_list_does_not_trigger_subnet_fallback() {
        let subnet = "192.168.31.8/30".parse::<Ipv4Net>().unwrap();
        let local_ip = "192.168.31.9".parse::<Ipv4Addr>().unwrap();
        let targets = normalize_probe_targets(Vec::new(), local_ip, subnet);
        assert!(targets.is_empty());
    }

    #[test]
    fn normalize_targets_keeps_stable_sorted_dedup_order() {
        let subnet = "192.168.31.0/24".parse::<Ipv4Net>().unwrap();
        let local_ip = "192.168.31.8".parse::<Ipv4Addr>().unwrap();
        let targets = normalize_probe_targets(
            vec![
                String::from("192.168.31.44"),
                String::from("192.168.31.12"),
                String::from("192.168.31.44"),
                String::from("192.168.31.100"),
            ],
            local_ip,
            subnet,
        );
        assert_eq!(
            targets,
            vec![
                "192.168.31.12".parse::<Ipv4Addr>().unwrap(),
                "192.168.31.44".parse::<Ipv4Addr>().unwrap(),
                "192.168.31.100".parse::<Ipv4Addr>().unwrap(),
            ]
        );
    }

    #[test]
    fn scan_limit_switches_to_burst_profile_for_large_subnets() {
        assert_eq!(
            scan_concurrency_limit(SCAN_HOST_BURST_THRESHOLD - 1),
            SCAN_CONCURRENCY
        );
        assert_eq!(
            scan_concurrency_limit(SCAN_HOST_BURST_THRESHOLD),
            SCAN_CONCURRENCY_BURST
        );
        assert_eq!(
            scan_worker_count(SCAN_CONCURRENCY_BURST + 20, SCAN_CONCURRENCY_BURST),
            SCAN_CONCURRENCY_BURST
        );
    }

    #[test]
    fn scan_timeout_switches_to_fast_profile_for_large_subnets() {
        assert_eq!(
            scan_timeout(SCAN_HOST_FAST_TIMEOUT_THRESHOLD - 1),
            SCAN_TIMEOUT
        );
        assert_eq!(
            scan_timeout(SCAN_HOST_FAST_TIMEOUT_THRESHOLD),
            SCAN_TIMEOUT_FAST
        );
        assert_eq!(
            scan_timeout(SCAN_HOST_FAST_TIMEOUT_THRESHOLD + 10),
            SCAN_TIMEOUT_FAST
        );
    }

    #[test]
    fn fallback_only_for_bind_related_errors() {
        let bind_error = std::io::Error::from(std::io::ErrorKind::AddrNotAvailable);
        let connect_error = std::io::Error::from(std::io::ErrorKind::ConnectionRefused);
        let route_error = std::io::Error::from(std::io::ErrorKind::NetworkUnreachable);
        let host_error = std::io::Error::from(std::io::ErrorKind::HostUnreachable);

        assert!(should_retry_without_bind(&bind_error));
        assert!(!should_retry_without_bind(&connect_error));
        assert!(!should_retry_without_bind(&route_error));
        assert!(!should_retry_without_bind(&host_error));
    }

    #[test]
    fn retry_only_for_timeout_like_probe_outcome() {
        let timeout_error = std::io::Error::from(std::io::ErrorKind::TimedOut);
        let refused_error = std::io::Error::from(std::io::ErrorKind::ConnectionRefused);
        let network_error = std::io::Error::from(std::io::ErrorKind::NetworkUnreachable);
        let host_error = std::io::Error::from(std::io::ErrorKind::HostUnreachable);

        assert_eq!(
            classify_connect_error(&timeout_error),
            ProbeOutcome::RetryableTimeout
        );
        assert_eq!(classify_connect_error(&refused_error), ProbeOutcome::Closed);
        assert_eq!(classify_connect_error(&network_error), ProbeOutcome::Closed);
        assert_eq!(classify_connect_error(&host_error), ProbeOutcome::Closed);
    }

    #[test]
    fn collect_retry_targets_only_keeps_retryable_timeout_hosts() {
        let ip1 = "192.168.1.10".parse::<Ipv4Addr>().unwrap();
        let ip2 = "192.168.1.11".parse::<Ipv4Addr>().unwrap();
        let ip3 = "192.168.1.12".parse::<Ipv4Addr>().unwrap();
        let results = vec![
            (ip1, ProbeOutcome::Open),
            (ip2, ProbeOutcome::RetryableTimeout),
            (ip3, ProbeOutcome::Closed),
        ];

        assert_eq!(collect_retry_targets(&results), vec![ip2]);
    }

    #[test]
    fn retry_worker_count_uses_burst_limit_for_large_retry_batches() {
        assert_eq!(retry_worker_count(0), 0);
        assert_eq!(retry_worker_count(10), 10);
        assert_eq!(
            retry_worker_count(RETRY_SCAN_CONCURRENCY),
            RETRY_SCAN_CONCURRENCY
        );
        assert_eq!(
            retry_worker_count(RETRY_HOST_BURST_THRESHOLD),
            RETRY_SCAN_CONCURRENCY_BURST
        );
        assert_eq!(
            retry_worker_count(RETRY_HOST_BURST_THRESHOLD + 20),
            RETRY_SCAN_CONCURRENCY_BURST
        );
    }

    #[test]
    fn retry_timeout_switches_to_fast_profile_for_large_retry_batches() {
        assert_eq!(
            retry_scan_timeout(RETRY_HOST_FAST_TIMEOUT_THRESHOLD - 1),
            RETRY_SCAN_TIMEOUT
        );
        assert_eq!(
            retry_scan_timeout(RETRY_HOST_FAST_TIMEOUT_THRESHOLD),
            RETRY_SCAN_TIMEOUT_FAST
        );
        assert_eq!(
            retry_scan_timeout(RETRY_HOST_FAST_TIMEOUT_THRESHOLD + 10),
            RETRY_SCAN_TIMEOUT_FAST
        );
    }

    #[test]
    fn progress_counts_retryable_timeout_only_in_retry_round() {
        assert!(!should_report_progress(
            ProbeOutcome::RetryableTimeout,
            false
        ));
        assert!(should_report_progress(ProbeOutcome::RetryableTimeout, true));
        assert!(should_report_progress(ProbeOutcome::Open, false));
        assert!(should_report_progress(ProbeOutcome::Closed, false));
    }

    #[test]
    fn refused_or_closed_outcomes_do_not_enter_retry_targets() {
        let refused_outcome =
            classify_connect_error(&std::io::Error::from(std::io::ErrorKind::ConnectionRefused));
        let timeout_outcome =
            classify_connect_error(&std::io::Error::from(std::io::ErrorKind::TimedOut));
        let results = vec![
            ("192.168.1.20".parse::<Ipv4Addr>().unwrap(), refused_outcome),
            (
                "192.168.1.21".parse::<Ipv4Addr>().unwrap(),
                ProbeOutcome::Closed,
            ),
            ("192.168.1.22".parse::<Ipv4Addr>().unwrap(), timeout_outcome),
        ];
        assert_eq!(
            collect_retry_targets(&results),
            vec!["192.168.1.22".parse::<Ipv4Addr>().unwrap()]
        );
    }

    #[test]
    fn open_host_result_is_sorted_and_deduplicated() {
        let mut open_hosts = vec![
            "192.168.31.44".parse::<Ipv4Addr>().unwrap(),
            "192.168.31.12".parse::<Ipv4Addr>().unwrap(),
            "192.168.31.44".parse::<Ipv4Addr>().unwrap(),
        ];
        sort_and_dedup_hosts(&mut open_hosts);
        assert_eq!(
            open_hosts,
            vec![
                "192.168.31.12".parse::<Ipv4Addr>().unwrap(),
                "192.168.31.44".parse::<Ipv4Addr>().unwrap(),
            ]
        );
    }

    #[test]
    fn probe_report_splits_candidates_into_open_closed_and_retry_exhausted() {
        let report = build_probe_report(
            vec![
                "192.168.31.12".parse::<Ipv4Addr>().unwrap(),
                "192.168.31.44".parse::<Ipv4Addr>().unwrap(),
                "192.168.31.50".parse::<Ipv4Addr>().unwrap(),
            ],
            vec!["192.168.31.44".parse::<Ipv4Addr>().unwrap()],
            vec![
                "192.168.31.12".parse::<Ipv4Addr>().unwrap(),
                "192.168.31.50".parse::<Ipv4Addr>().unwrap(),
            ],
        );

        assert_eq!(
            report.candidate_hosts,
            vec!["192.168.31.12", "192.168.31.44", "192.168.31.50"]
        );
        assert_eq!(report.open_hosts, vec!["192.168.31.44"]);
        assert_eq!(report.closed_hosts, vec!["192.168.31.12", "192.168.31.50"]);
        assert_eq!(
            report.retry_exhausted_hosts,
            vec!["192.168.31.12", "192.168.31.50"]
        );
    }

    #[tokio::test]
    async fn zero_candidates_finish_cleanly_and_report_zero_progress() {
        let progress = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = Arc::clone(&progress);
        let result = scan_subnet(
            "192.168.31.8",
            "192.168.31.0/24",
            Vec::new(),
            move |scanned, total| {
                captured.lock().unwrap().push((scanned, total));
            },
            CancellationToken::new(),
        )
        .await;

        assert!(result.is_empty());
        assert_eq!(progress.lock().unwrap().as_slice(), &[(0, 0)]);
    }
}
