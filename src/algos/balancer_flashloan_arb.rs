use std::sync::Arc;

use ethers::providers::{Middleware, Provider, PubsubClient};

use crate::world::WorldState;

struct BalancerFlashloanArb<M, P> {
    provider: Arc<Provider<P>>,
    world_state: Arc<WorldState<M, P>>,
}

impl<M: Middleware + Clone, P: PubsubClient> BalancerFlashloanArb<M, P> {}
