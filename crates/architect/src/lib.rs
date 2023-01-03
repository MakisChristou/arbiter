#![warn(missing_docs)]
#![warn(unsafe_code)]

//! ## Architect
//!
//! Architect is the bundling, simulation and execution module of Arbiter.

use ethers::core::rand::thread_rng;
use ethers::core::types::transaction::eip2718::TypedTransaction;
use ethers::prelude::*;
use ethers::signers::Signer;
use ethers_flashbots::*;
use url::Url;

/// Type that represents an `Architect`, a transaction executor designed to
/// execute, simulate and bundle arbitrage opportunities.
#[derive(Debug)]
pub struct Architect<S>
where
    S: Signer,
{
    /// Client that signs transactions.
    pub client: SignerMiddleware<FlashbotsMiddleware<Provider<Http>, LocalWallet>, S>,
    /// Bundle to be executed.
    pub bundle: BundleRequest,
}

/// Type that represents an execution result from either a send or simulation.
pub type ExecutionResult<T> = Result<T, FlashbotsMiddlewareError<Provider<Http>, LocalWallet>>;

impl<S: Signer> Architect<S> {
    /// Public constructor function that instantiates an `Architect`.
    pub async fn new(provider: Provider<Http>, wallet: S) -> Self {
        // This is your searcher identity.
        // It does not store funds and is not used for transaction execution.
        let bundle_signer = LocalWallet::new(&mut thread_rng());
        let bundle = BundleRequest::new();
        let client = SignerMiddleware::new(
            FlashbotsMiddleware::new(
                provider,
                Url::parse("https://relay.flashbots.net").unwrap(),
                bundle_signer,
            ),
            wallet,
        );

        let block_number = client.get_block_number().await.unwrap();

        Self {
            client,
            bundle: bundle
                .set_block(block_number + 1)
                .set_simulation_block(block_number),
        }
    }

    /// Add and sign a transaction to the bundle to be executed.
    pub async fn add_transactions(mut self, transactions: &Vec<TypedTransaction>) -> Self {
        for tx in transactions {
            let signature = self.client.signer().sign_transaction(tx).await.unwrap();

            self.bundle = self.bundle.push_transaction(tx.rlp_signed(&signature));
        }

        self
    }

    /// Simulate bundle execution.
    pub async fn simulate(&mut self) -> ExecutionResult<SimulatedBundle> {
        self.client.inner().simulate_bundle(&self.bundle).await
    }

    /// Send the bundle.
    pub async fn send(
        &mut self,
    ) -> ExecutionResult<
        PendingBundle<
            '_,
            <FlashbotsMiddleware<Provider<Http>, LocalWallet> as Middleware>::Provider,
        >,
    > {
        self.client.inner().send_bundle(&self.bundle).await
    }
}

#[cfg(test)]
mod tests {
    use crate::Architect;
    use ethers::core::rand::thread_rng;
    use ethers::prelude::*;
    use ethers::types::transaction::eip2718::TypedTransaction;

    // We will need more tests in future but this just ensures basic functionality is working.
    #[tokio::test]
    async fn test_architect_creation() {
        let provider = Provider::<Http>::try_from("https://mainnet.eth.aragon.network").unwrap();
        let tx = TypedTransaction::Legacy(TransactionRequest::pay("vitalik.eth", 100));

        let mut architect = Architect::new(provider, LocalWallet::new(&mut thread_rng()))
            .await
            .add_transactions(&vec![tx])
            .await;
        architect.simulate().await;
    }
}
