use super::auth::load_or_create_token;
use super::http::{router, LocalAgentHttpState};
use anyhow::{Context, Result};
use bitfun_core::agentic::coordination::{ConversationCoordinator, DialogScheduler};
use bitfun_core::agentic::local_agent_api::{LocalAgentApiService, TaskResultTracker};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

pub const DEFAULT_LOCAL_AGENT_API_PORT: u16 = 17_373;

pub async fn start_local_agent_api_server(
    config_path: PathBuf,
    coordinator: Arc<ConversationCoordinator>,
    scheduler: Arc<DialogScheduler>,
    tracker: Arc<TaskResultTracker>,
) -> Result<()> {
    let token = load_or_create_token(config_path).await?;
    let service = Arc::new(LocalAgentApiService::new(coordinator, scheduler, tracker));
    let state = LocalAgentHttpState {
        service,
        token: Arc::new(token),
    };
    let app = router(state);
    let addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        DEFAULT_LOCAL_AGENT_API_PORT,
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind Local Agent API server at {}", addr))?;

    log::info!("Local Agent API server started at http://{}", addr);
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            log::error!("Local Agent API server stopped with error: {}", error);
        }
    });

    Ok(())
}
