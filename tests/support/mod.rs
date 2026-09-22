// Explicit opt-in for deterministic test doubles that perform no provider HTTP.
macro_rules! controlled_local_backend {
    () => {
        fn complete_with_request_control(
            &self,
            request: zenpi::backend::CompletionRequest<'_>,
            control: &mut zenpi::backend::RequestControl<'_>,
            sink: &mut dyn FnMut(
                zenpi::backend::ProviderEvent,
            ) -> Result<(), zenpi::backend::BackendError>,
        ) -> Result<zenpi::backend::Completion, zenpi::backend::BackendError> {
            control.check_cancelled()?;
            let result = self.complete_with_control(
                request,
                &|| control.check_cancelled().is_err(),
                &mut |event| {
                    control.check_cancelled()?;
                    sink(event)
                },
            );
            control.check_cancelled()?;
            result
        }
    };
}

pub(crate) use controlled_local_backend;
