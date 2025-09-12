use tunnels::TunnelType;

#[derive(Debug, Clone)]
pub struct CoodinatorRules {
    pub filter: Option<CoordinatorNodeFilter>,
}

#[derive(Debug, Clone)]
pub struct CoordinatorNodeFilter {
    pub tunnels: Vec<TunnelType>,
    pub nodes: u64,
}
