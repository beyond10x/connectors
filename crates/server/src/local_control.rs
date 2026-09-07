//! Runtime bindings for local process control and confidential enrollment.

use super::{Arc, ConnectorBackend, LocalOperationDaemon, PathBuf};

impl<B: ConnectorBackend + ?Sized> LocalOperationDaemon<B> {
    /// Name the exact configuration served, so background startup detects another configuration.
    #[must_use]
    pub fn with_configuration(mut self, configuration: Option<PathBuf>) -> Self {
        self.lifecycle = Arc::new(crate::local_lifecycle::Lifecycle::new(configuration));
        self
    }

    /// Bind confidential enrollment to the runtime's own configuration and credential store.
    #[must_use]
    pub fn with_setup_handler(
        mut self,
        handler: Arc<dyn crate::local_setup::LocalSetupHandler>,
    ) -> Self {
        self.setup = Some(handler);
        self
    }
}
